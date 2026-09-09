//! Isolated Open303 candidate. Available only with the `open303` feature.
//!
//! Owns an exclusively accessed C++ core prepared outside rendering. No plugin,
//! audio server, preset schema, or production model integration is involved.
//! Controls are direct event-time settings; this baseline does not add smoothing.
//! Output uses a documented ±0.999 presentation ceiling, with counted contact.
use std::{cell::Cell, ffi::c_void, marker::PhantomData, ptr::NonNull};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum FilterMode {
    Tb303 = 0,
    Lowpass18 = 1,
}

/// Physical units, not the production macro/control schema.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Controls {
    pub waveform: f64,
    pub cutoff_hz: f64,
    pub resonance: f64,
    pub env_mod: f64,
    pub decay_ms: f64,
    pub accent: f64,
    pub volume_db: f64,
    pub tuning_hz: f64,
    pub slide_ms: f64,
}
impl Default for Controls {
    fn default() -> Self {
        // Project-authored neutral candidate settings, not an imported factory patch.
        Self {
            waveform: 0.5,
            cutoff_hz: 700.0,
            resonance: 40.0,
            env_mod: 35.0,
            decay_ms: 500.0,
            accent: 60.0,
            volume_db: -24.0,
            tuning_hz: 440.0,
            slide_ms: 60.0,
        }
    }
}
impl Controls {
    fn validate(self) -> Result<(), Open303Error> {
        let fields = [
            (self.waveform, 0.0, 1.0),
            (self.cutoff_hz, 314.0, 2394.0),
            (self.resonance, 0.0, 100.0),
            (self.env_mod, 0.0, 100.0),
            (self.decay_ms, 200.0, 2000.0),
            (self.accent, 0.0, 100.0),
            (self.volume_db, -60.0, 0.0),
            (self.tuning_hz, 400.0, 480.0),
            (self.slide_ms, 0.0, 300.0),
        ];
        if fields
            .into_iter()
            .all(|(v, lo, hi)| v.is_finite() && (lo..=hi).contains(&v))
        {
            Ok(())
        } else {
            Err(Open303Error::InvalidControls)
        }
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum Open303Error {
    #[error("Open303 candidate requires a finite sample rate from 44100 to 96000 Hz")]
    InvalidSampleRate,
    #[error("Open303 control is non-finite or outside its documented range")]
    InvalidControls,
    #[error("MIDI key and velocity must be in 0..=127")]
    InvalidNote,
    #[error("Open303 core preparation failed")]
    PreparationFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderStatus {
    pub clipped_samples: u64,
    /// A non-finite internal sample latches silence until explicit reset.
    pub faulted: bool,
}
#[repr(C)]
struct NativeStatus {
    clipped_samples: u64,
    faulted: u32,
}

unsafe extern "C" {
    fn moj_open303_create(rate: f64, mode: i32, controls: *const Controls) -> *mut c_void;
    fn moj_open303_destroy(voice: *mut c_void);
    fn moj_open303_controls(voice: *mut c_void, controls: *const Controls) -> i32;
    fn moj_open303_note(voice: *mut c_void, key: i32, velocity: i32) -> i32;
    fn moj_open303_articulation(
        voice: *mut c_void,
        normal: f64,
        attack: f64,
        decay: f64,
        amp: f64,
    ) -> i32;
    fn moj_open303_release_all(voice: *mut c_void);
    fn moj_open303_reset(voice: *mut c_void);
    fn moj_open303_idle(voice: *const c_void) -> i32;
    fn moj_open303_render(voice: *mut c_void, output: *mut f32, frames: usize);
    fn moj_open303_status(voice: *const c_void) -> NativeStatus;
}

/// Unique ownership; destruction frees native storage and belongs off the callback.
/// No Clone or Sync: the native core contains self-references and mutable DSP state.
#[derive(Debug)]
pub struct Open303 {
    native: NonNull<c_void>,
    controls: Controls,
    _not_sync: PhantomData<Cell<()>>,
}
// SAFETY: ownership may move between threads. Table preparation has no shared
// mutable scratch; all subsequent native access requires exclusive Rust access.
unsafe impl Send for Open303 {}

impl Open303 {
    pub fn new(
        sample_rate: f64,
        mode: FilterMode,
        controls: Controls,
    ) -> Result<Self, Open303Error> {
        if !sample_rate.is_finite() || !(44100.0..=96000.0).contains(&sample_rate) {
            return Err(Open303Error::InvalidSampleRate);
        }
        controls.validate()?;
        // SAFETY: repr(C) controls lives throughout this synchronous call;
        // create catches exceptions and returns uniquely owned stable storage.
        let native =
            NonNull::new(unsafe { moj_open303_create(sample_rate, mode as i32, &controls) })
                .ok_or(Open303Error::PreparationFailed)?;
        Ok(Self {
            native,
            controls,
            _not_sync: PhantomData,
        })
    }

    /// Native envelope timings in milliseconds; validated atomically by the C boundary.
    pub fn set_articulation(
        &mut self,
        normal: f64,
        attack: f64,
        decay: f64,
        amp: f64,
    ) -> Result<(), Open303Error> {
        // SAFETY: exclusive live ownership, bounded nonallocating native setter.
        if unsafe { moj_open303_articulation(self.native.as_ptr(), normal, attack, decay, amp) }
            == 0
        {
            return Err(Open303Error::InvalidControls);
        }
        Ok(())
    }

    pub fn controls(&self) -> Controls {
        self.controls
    }

    pub fn set_controls(&mut self, controls: Controls) -> Result<(), Open303Error> {
        controls.validate()?;
        // SAFETY: uniquely borrowed live core; valid C-layout value; setter does not allocate.
        let accepted = unsafe { moj_open303_controls(self.native.as_ptr(), &controls) };
        if accepted == 0 {
            return Err(Open303Error::InvalidControls);
        }
        self.controls = controls;
        Ok(())
    }

    /// Zero velocity releases the key. Velocity >= 100 selects accent.
    pub fn note_on(&mut self, key: u8, velocity: u8) -> Result<(), Open303Error> {
        if key > 127 || velocity > 127 {
            return Err(Open303Error::InvalidNote);
        }
        // SAFETY: live exclusively borrowed object and bounded scalar MIDI values.
        unsafe {
            moj_open303_note(self.native.as_ptr(), i32::from(key), i32::from(velocity));
        }
        Ok(())
    }

    /// Unknown or out-of-range key releases are ignored.
    pub fn note_off(&mut self, key: u8) {
        if key <= 127 {
            // SAFETY: same exclusive ownership as note_on; zero means release.
            unsafe {
                moj_open303_note(self.native.as_ptr(), i32::from(key), 0);
            }
        }
    }

    /// Release every held key through the inherited amplitude envelope.
    pub fn release_all(&mut self) {
        // SAFETY: native function mutates only the exclusively borrowed core.
        unsafe {
            moj_open303_release_all(self.native.as_ptr());
        }
    }

    /// Immediate silent reset, retaining prepared tables and configured controls.
    pub fn reset(&mut self) {
        // SAFETY: reset clears state in place without allocation or destruction.
        unsafe {
            moj_open303_reset(self.native.as_ptr());
        }
    }

    pub fn is_idle(&self) -> bool {
        // SAFETY: read-only access; the wrapper is not Sync.
        unsafe { moj_open303_idle(self.native.as_ptr()) != 0 }
    }

    pub fn status(&self) -> RenderStatus {
        // SAFETY: live read-only object; NativeStatus matches the private C ABI.
        let status = unsafe { moj_open303_status(self.native.as_ptr()) };
        RenderStatus {
            clipped_samples: status.clipped_samples,
            faulted: status.faulted != 0,
        }
    }

    /// Render mono into caller-owned memory. Split buffers at event boundaries.
    pub fn render(&mut self, output: &mut [f32]) {
        // SAFETY: output is writable for exactly len elements and cannot alias
        // the separately owned core. Native render retains no pointer.
        unsafe {
            moj_open303_render(self.native.as_ptr(), output.as_mut_ptr(), output.len());
        }
    }
}
impl Drop for Open303 {
    fn drop(&mut self) {
        // SAFETY: this is the sole owner; destruction runs once and never unwinds.
        unsafe {
            moj_open303_destroy(self.native.as_ptr());
        }
    }
}
