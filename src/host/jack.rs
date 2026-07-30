use super::queue::Consumer;
use crate::engine::{Engine, Event, TimedEvent};
use crate::preset::Preset;
use anyhow::{Context, Result, bail};
use libc::{c_char, c_int, c_uint, c_ulong, c_void};
use std::ffi::CString;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

const JACK_DEFAULT_AUDIO_TYPE: &[u8] = b"32 bit float mono audio\0";
const JACK_PORT_IS_OUTPUT: c_ulong = 2;
const JACK_NO_START_SERVER: c_uint = 1;
const MAX_PERIOD_EVENTS: usize = 256;

#[repr(C)]
struct OpaqueClient {
    _private: [u8; 0],
}
#[repr(C)]
struct Port {
    _private: [u8; 0],
}

type ClientOpen = unsafe extern "C" fn(*const c_char, c_uint, *mut c_uint) -> *mut OpaqueClient;
type ClientClose = unsafe extern "C" fn(*mut OpaqueClient) -> c_int;
type PortRegister = unsafe extern "C" fn(
    *mut OpaqueClient,
    *const c_char,
    *const c_char,
    c_ulong,
    c_ulong,
) -> *mut Port;
type SetProcess = unsafe extern "C" fn(*mut OpaqueClient, ProcessCallback, *mut c_void) -> c_int;
type OnShutdown = unsafe extern "C" fn(*mut OpaqueClient, ShutdownCallback, *mut c_void);
type Activate = unsafe extern "C" fn(*mut OpaqueClient) -> c_int;
type Deactivate = unsafe extern "C" fn(*mut OpaqueClient) -> c_int;
type SampleRate = unsafe extern "C" fn(*const OpaqueClient) -> c_uint;
type PortGetBuffer = unsafe extern "C" fn(*mut Port, c_uint) -> *mut c_void;
type FramesSinceCycleStart = unsafe extern "C" fn(*const OpaqueClient) -> c_uint;
type ProcessCallback = unsafe extern "C" fn(c_uint, *mut c_void) -> c_int;
type ShutdownCallback = unsafe extern "C" fn(*mut c_void);

#[derive(Clone, Copy, Debug)]
pub struct ScheduledEvent {
    pub cycle: u64,
    pub sample_offset: usize,
    pub event: Event,
}

#[derive(Clone)]
pub struct Timing {
    client: *mut OpaqueClient,
    frames_since_cycle_start: FramesSinceCycleStart,
    cycle: Arc<AtomicU64>,
}
unsafe impl Send for Timing {}

impl Timing {
    pub fn schedule(&self, event: Event) -> ScheduledEvent {
        let cycle = self.cycle.load(Ordering::Acquire).saturating_add(1);
        // JACK documents this query for non-process threads; it does not
        // change the graph or server configuration.
        let sample_offset = unsafe { (self.frames_since_cycle_start)(self.client) as usize };
        ScheduledEvent {
            cycle,
            sample_offset,
            event,
        }
    }
}

#[derive(Clone, Copy)]
struct Api {
    handle: *mut c_void,
    close: ClientClose,
    register: PortRegister,
    set_process: SetProcess,
    on_shutdown: OnShutdown,
    activate: Activate,
    deactivate: Deactivate,
    sample_rate: SampleRate,
    get_buffer: PortGetBuffer,
    frames_since_cycle_start: FramesSinceCycleStart,
}

struct CallbackState {
    engine: Engine,
    queue: Consumer<ScheduledEvent>,
    pending: Option<ScheduledEvent>,
    events: [TimedEvent; MAX_PERIOD_EVENTS],
    left: *mut Port,
    right: *mut Port,
    get_buffer: PortGetBuffer,
    cycle: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
    jack_failed: Arc<AtomicBool>,
    callback_overflow: Arc<AtomicU64>,
}

pub struct JackHost {
    client: *mut OpaqueClient,
    api: Api,
    _state: Box<CallbackState>,
    active: bool,
}

unsafe impl Send for JackHost {}

impl JackHost {
    pub fn open(
        client_name: &str,
        preset: &Preset,
        queue: Consumer<ScheduledEvent>,
        shutdown: Arc<AtomicBool>,
        jack_failed: Arc<AtomicBool>,
        callback_overflow: Arc<AtomicU64>,
    ) -> Result<(Self, Timing)> {
        let name = CString::new(client_name).context("JACK client name contains a NUL byte")?;
        // SAFETY: every symbol remains backed by the retained dlopen handle.
        unsafe {
            let handle = libc::dlopen(c"libjack.so.0".as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
            if handle.is_null() {
                bail!("JACK library libjack.so.0 is unavailable");
            }
            let loaded = (|| -> Result<(ClientOpen, Api)> {
                Ok((
                    symbol(handle, b"jack_client_open\0")?,
                    Api {
                        handle,
                        close: symbol(handle, b"jack_client_close\0")?,
                        register: symbol(handle, b"jack_port_register\0")?,
                        set_process: symbol(handle, b"jack_set_process_callback\0")?,
                        on_shutdown: symbol(handle, b"jack_on_shutdown\0")?,
                        activate: symbol(handle, b"jack_activate\0")?,
                        deactivate: symbol(handle, b"jack_deactivate\0")?,
                        sample_rate: symbol(handle, b"jack_get_sample_rate\0")?,
                        get_buffer: symbol(handle, b"jack_port_get_buffer\0")?,
                        frames_since_cycle_start: symbol(
                            handle,
                            b"jack_frames_since_cycle_start\0",
                        )?,
                    },
                ))
            })();
            let (open, api) = match loaded {
                Ok(loaded) => loaded,
                Err(error) => {
                    libc::dlclose(handle);
                    return Err(error);
                }
            };
            let mut status = 0;
            let client = open(name.as_ptr(), JACK_NO_START_SERVER, &mut status);
            if client.is_null() {
                libc::dlclose(handle);
                bail!(
                    "JACK server/client setup failed without starting a server (status {status})"
                );
            }
            let result = (|| -> Result<(Self, Timing)> {
                let left = register_output(&api, client, "out_l")?;
                let right = register_output(&api, client, "out_r")?;
                let sample_rate = (api.sample_rate)(client);
                let engine = Engine::new(sample_rate as f32, preset)
                    .context("prepare Moj Sint engine for JACK sample rate")?;
                let cycle = Arc::new(AtomicU64::new(0));
                let timing = Timing {
                    client,
                    frames_since_cycle_start: api.frames_since_cycle_start,
                    cycle: cycle.clone(),
                };
                let mut state = Box::new(CallbackState {
                    engine,
                    queue,
                    pending: None,
                    events: [TimedEvent::new(0, Event::AllNotesOff); MAX_PERIOD_EVENTS],
                    left,
                    right,
                    get_buffer: api.get_buffer,
                    cycle,
                    shutdown,
                    jack_failed,
                    callback_overflow,
                });
                if (api.set_process)(
                    client,
                    process_callback,
                    (&mut *state as *mut CallbackState).cast(),
                ) != 0
                {
                    bail!("register JACK process callback");
                }
                (api.on_shutdown)(
                    client,
                    shutdown_callback,
                    (&mut *state as *mut CallbackState).cast(),
                );
                Ok((
                    Self {
                        client,
                        api,
                        _state: state,
                        active: false,
                    },
                    timing,
                ))
            })();
            match result {
                Ok(value) => Ok(value),
                Err(error) => {
                    (api.close)(client);
                    libc::dlclose(api.handle);
                    Err(error)
                }
            }
        }
    }

    pub fn activate(&mut self) -> Result<()> {
        if unsafe { (self.api.activate)(self.client) } != 0 {
            bail!("activate JACK client");
        }
        self.active = true;
        Ok(())
    }
}

impl Drop for JackHost {
    fn drop(&mut self) {
        if self.active {
            unsafe { (self.api.deactivate)(self.client) };
        }
        unsafe {
            (self.api.close)(self.client);
            libc::dlclose(self.api.handle);
        }
    }
}

unsafe fn symbol<T: Copy>(handle: *mut c_void, name: &[u8]) -> Result<T> {
    let pointer = unsafe { libc::dlsym(handle, name.as_ptr().cast()) };
    if pointer.is_null() {
        bail!(
            "JACK symbol {} is unavailable",
            String::from_utf8_lossy(&name[..name.len().saturating_sub(1)])
        );
    }
    Ok(unsafe { std::mem::transmute_copy(&pointer) })
}

unsafe fn register_output(api: &Api, client: *mut OpaqueClient, name: &str) -> Result<*mut Port> {
    let name = CString::new(name)?;
    let port = unsafe {
        (api.register)(
            client,
            name.as_ptr(),
            JACK_DEFAULT_AUDIO_TYPE.as_ptr().cast(),
            JACK_PORT_IS_OUTPUT,
            0,
        )
    };
    if port.is_null() {
        bail!("register JACK output {}", name.to_string_lossy());
    }
    Ok(port)
}

unsafe extern "C" fn shutdown_callback(argument: *mut c_void) {
    if let Some(state) = unsafe { argument.cast::<CallbackState>().as_ref() } {
        state.jack_failed.store(true, Ordering::Release);
        state.shutdown.store(true, Ordering::Release);
    }
}

unsafe extern "C" fn process_callback(frames: c_uint, argument: *mut c_void) -> c_int {
    let Some(state) = (unsafe { argument.cast::<CallbackState>().as_mut() }) else {
        return 0;
    };
    let frames = frames as usize;
    if frames == 0 {
        return 0;
    }
    let left = unsafe { (state.get_buffer)(state.left, frames as c_uint).cast::<f32>() };
    let right = unsafe { (state.get_buffer)(state.right, frames as c_uint).cast::<f32>() };
    if left.is_null() || right.is_null() {
        state.shutdown.store(true, Ordering::Release);
        return 0;
    }
    let left = unsafe { std::slice::from_raw_parts_mut(left, frames) };
    let right = unsafe { std::slice::from_raw_parts_mut(right, frames) };
    let cycle = state.cycle.fetch_add(1, Ordering::AcqRel) + 1;
    let mut count = 0;
    while let Some(scheduled) = state.pending.take().or_else(|| state.queue.pop()) {
        if scheduled.cycle > cycle {
            state.pending = Some(scheduled);
            break;
        }
        if count == MAX_PERIOD_EVENTS {
            state.callback_overflow.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        let offset = period_offset(scheduled, cycle, frames);
        let mut insert = count;
        while insert > 0 && state.events[insert - 1].sample_offset > offset {
            state.events[insert] = state.events[insert - 1];
            insert -= 1;
        }
        state.events[insert] = TimedEvent::new(offset, scheduled.event);
        count += 1;
    }
    if state
        .engine
        .render_block(&state.events[..count], left, right)
        .is_err()
    {
        left.fill(0.0);
        right.fill(0.0);
        state.jack_failed.store(true, Ordering::Release);
        state.shutdown.store(true, Ordering::Release);
    }
    0
}

fn period_offset(scheduled: ScheduledEvent, current_cycle: u64, frames: usize) -> usize {
    if scheduled.cycle < current_cycle {
        0
    } else {
        scheduled.sample_offset.min(frames.saturating_sub(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduled_event_is_fixed_size_copy_data() {
        fn copy<T: Copy>(value: T) -> T {
            value
        }
        let event = ScheduledEvent {
            cycle: 2,
            sample_offset: 17,
            event: Event::AllNotesOff,
        };
        assert_eq!(copy(event).sample_offset, 17);
    }

    #[test]
    fn period_timing_preserves_current_offsets_and_clamps_late_or_large_events() {
        let event = ScheduledEvent {
            cycle: 4,
            sample_offset: 17,
            event: Event::AllNotesOff,
        };
        assert_eq!(period_offset(event, 4, 64), 17);
        assert_eq!(period_offset(event, 5, 64), 0);
        assert_eq!(
            period_offset(
                ScheduledEvent {
                    sample_offset: 99,
                    ..event
                },
                4,
                64,
            ),
            63
        );
    }
}
