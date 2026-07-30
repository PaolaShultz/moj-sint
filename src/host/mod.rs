mod jack;
pub mod midi;
pub mod queue;

use self::jack::{JackHost, ScheduledEvent};
use self::queue::channel;
use crate::preset::Preset;
use alsa::Direction;
use alsa::seq::{PortCap, PortType, Seq};
use anyhow::{Context, Result};
use std::ffi::CString;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

pub fn run(client_name: &str, preset: &Preset) -> Result<()> {
    let stop = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, stop.clone())?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, stop.clone())?;

    let (producer, consumer) = channel::<ScheduledEvent>();
    let callback_overflow = Arc::new(AtomicU64::new(0));
    let jack_failed = Arc::new(AtomicBool::new(false));
    let (mut jack, timing) = JackHost::open(
        client_name,
        preset,
        consumer,
        stop.clone(),
        jack_failed.clone(),
        callback_overflow.clone(),
    )?;

    let seq = Seq::open(None, Some(Direction::Capture), true)
        .context("open ALSA Sequencer MIDI input")?;
    seq.set_client_name(&CString::new(client_name).context("ALSA client name contains NUL")?)
        .context("set ALSA Sequencer client name")?;
    seq.create_simple_port(
        &CString::new("input").expect("static port name"),
        PortCap::WRITE | PortCap::SUBS_WRITE,
        PortType::MIDI_GENERIC | PortType::APPLICATION,
    )
    .context("create ALSA Sequencer MIDI input port")?;

    let midi_stop = stop.clone();
    let midi_failed = Arc::new(AtomicBool::new(false));
    let midi_failed_thread = midi_failed.clone();
    let midi_thread = thread::Builder::new()
        .name("moj-sint-midi".into())
        .spawn(move || {
            let mut input = seq.input();
            while !midi_stop.load(Ordering::Acquire) {
                match input.event_input_pending(true) {
                    Ok(pending) if pending > 0 => match input.event_input() {
                        Ok(event) => {
                            if let Some(event) = midi::translate(&event) {
                                producer.push(timing.schedule(event));
                            }
                        }
                        Err(_) => {
                            midi_failed_thread.store(true, Ordering::Release);
                            midi_stop.store(true, Ordering::Release);
                        }
                    },
                    Ok(_) => thread::sleep(Duration::from_millis(1)),
                    Err(_) => {
                        midi_failed_thread.store(true, Ordering::Release);
                        midi_stop.store(true, Ordering::Release);
                    }
                }
            }
            producer.overflow_count()
        })
        .context("start ALSA MIDI input thread")?;

    if let Err(error) = jack.activate() {
        stop.store(true, Ordering::Release);
        drop(jack);
        let _ = midi_thread.join();
        return Err(error);
    }
    let mut last_overflow = 0;
    while !stop.load(Ordering::Acquire) {
        thread::sleep(Duration::from_millis(100));
        let current = callback_overflow.load(Ordering::Relaxed);
        if current != last_overflow {
            eprintln!("Moj Sint callback event overflow count: {current}");
            last_overflow = current;
        }
    }
    drop(jack);
    let queue_overflow = midi_thread
        .join()
        .map_err(|_| anyhow::anyhow!("ALSA MIDI input thread panicked"))?;
    if queue_overflow > 0 {
        eprintln!("Moj Sint MIDI queue overflow count: {queue_overflow}");
    }
    if jack_failed.load(Ordering::Acquire) {
        anyhow::bail!("JACK shut down while Moj Sint was active");
    }
    if midi_failed.load(Ordering::Acquire) {
        anyhow::bail!("ALSA Sequencer MIDI input failed while Moj Sint was active");
    }
    Ok(())
}
