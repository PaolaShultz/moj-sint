//! Native callback-simulation support for Raspberry Pi performance evidence.
//!
//! This module deliberately does not host JACK or ALSA. Construction,
//! platform inspection, timing, statistics, and report I/O happen outside the
//! render boundary. `PreparedCase::render_callback` itself uses only prepared
//! fixed work, bounded borrowed events, and caller-owned output buffers.

use crate::control::{MacroId, Normalized};
use crate::engine::{Engine, EngineError, Event, TimedEvent};
use crate::model_d::ModelDError;
use crate::model_d::voice::ModelDVoice;
use crate::model_d_lab::idealized_bass_voice;
use crate::preset::{Preset, PresetError};
use std::fs;
use std::process::Command;
use thiserror::Error;

const MAX_BENCH_VOICES: usize = 8;
const MAX_EVENTS_PER_BLOCK: usize = 16;
const EVENT_CYCLE_BLOCKS: usize = 32;
const CHORD_NOTES: [u8; MAX_BENCH_VOICES] = [36, 43, 48, 52, 55, 60, 64, 67];
const STEAL_NOTES: [u8; 16] = [
    36, 43, 48, 52, 55, 60, 64, 67, 70, 74, 77, 81, 84, 79, 72, 65,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderPath {
    ProductionEngine,
    ModelDIdealized,
}

impl RenderPath {
    pub const ALL: [Self; 2] = [Self::ProductionEngine, Self::ModelDIdealized];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProductionEngine => "production-engine",
            Self::ModelDIdealized => "model-d-idealized",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scenario {
    HeldNote,
    FullChord,
    VoiceStealing,
    RapidControl,
}

impl Scenario {
    pub const ALL: [Self; 4] = [
        Self::HeldNote,
        Self::FullChord,
        Self::VoiceStealing,
        Self::RapidControl,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HeldNote => "held-note",
            Self::FullChord => "full-chord",
            Self::VoiceStealing => "voice-stealing",
            Self::RapidControl => "rapid-control",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EngineMacroConfig {
    pub shape: f32,
    pub color: f32,
    pub edge: f32,
    pub couple: f32,
}

impl EngineMacroConfig {
    pub const MIDPOINT: Self = Self {
        shape: 0.5,
        color: 0.5,
        edge: 0.5,
        couple: 0.5,
    };

    pub fn validate(self) -> bool {
        [self.shape, self.color, self.edge, self.couple]
            .into_iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(&value))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaseConfig {
    pub path: RenderPath,
    pub scenario: Scenario,
    pub sample_rate: u32,
    pub frames: usize,
    pub voices: usize,
    pub engine_macros: EngineMacroConfig,
}

impl CaseConfig {
    pub const fn is_applicable(self) -> bool {
        !matches!(
            (self.path, self.scenario),
            (RenderPath::ModelDIdealized, Scenario::RapidControl)
        )
    }
}

#[derive(Debug, Error)]
pub enum BenchError {
    #[error("benchmark configuration is invalid")]
    InvalidConfig,
    #[error("rapid smoothed controls do not exist on the isolated Model-D candidate")]
    UnsupportedCandidateControlScenario,
    #[error("production Engine failed: {0}")]
    Engine(#[from] EngineError),
    #[error("production preset failed: {0}")]
    Preset(#[from] PresetError),
    #[error("Model-D voice failed: {0}")]
    ModelD(#[from] ModelDError),
    #[error("platform inspection failed: {0}")]
    Platform(#[from] std::io::Error),
    #[error("timing sample set is empty or the period is zero")]
    EmptyTiming,
}

#[derive(Clone, Debug)]
struct EngineSchedule {
    initial: Box<[TimedEvent]>,
    recurring: Vec<Box<[TimedEvent]>>,
    maximum_offset: usize,
    maximum_events: usize,
}

impl EngineSchedule {
    fn new(config: CaseConfig) -> Result<Self, BenchError> {
        let initial_notes = active_notes(config.scenario, config.voices);
        let mut initial = Vec::with_capacity(MAX_EVENTS_PER_BLOCK);
        for note in initial_notes {
            initial.push(TimedEvent::new(
                0,
                Event::NoteOn {
                    note,
                    velocity: 0.82,
                },
            ));
        }
        if config.scenario == Scenario::RapidControl {
            append_engine_control_events(&mut initial, config.frames, false)?;
        }

        let mut recurring = Vec::with_capacity(EVENT_CYCLE_BLOCKS);
        for cycle in 0..EVENT_CYCLE_BLOCKS {
            let mut events = Vec::with_capacity(MAX_EVENTS_PER_BLOCK);
            match config.scenario {
                Scenario::VoiceStealing if cycle % 4 == 0 => {
                    let note_index = (cycle / 4) % STEAL_NOTES.len();
                    events.push(TimedEvent::new(
                        0,
                        Event::NoteOff {
                            note: STEAL_NOTES[note_index],
                        },
                    ));
                    events.push(TimedEvent::new(
                        config.frames / 2,
                        Event::NoteOn {
                            note: STEAL_NOTES[(note_index + config.voices + 1) % STEAL_NOTES.len()],
                            velocity: 0.78,
                        },
                    ));
                }
                Scenario::RapidControl => {
                    append_engine_control_events(&mut events, config.frames, cycle % 2 != 0)?;
                }
                _ => {}
            }
            recurring.push(events.into_boxed_slice());
        }
        Self::finish(initial.into_boxed_slice(), recurring, config.frames)
    }

    fn finish(
        initial: Box<[TimedEvent]>,
        recurring: Vec<Box<[TimedEvent]>>,
        frames: usize,
    ) -> Result<Self, BenchError> {
        let maximum_offset = initial
            .iter()
            .chain(recurring.iter().flat_map(|events| events.iter()))
            .map(|event| event.sample_offset)
            .max()
            .unwrap_or(0);
        let maximum_events = recurring
            .iter()
            .map(|events| events.len())
            .chain(std::iter::once(initial.len()))
            .max()
            .unwrap_or(0);
        if maximum_offset >= frames || maximum_events > MAX_EVENTS_PER_BLOCK {
            return Err(BenchError::InvalidConfig);
        }
        Ok(Self {
            initial,
            recurring,
            maximum_offset,
            maximum_events,
        })
    }

    fn events(&self, callback_index: u64) -> &[TimedEvent] {
        if callback_index == 0 {
            &self.initial
        } else {
            &self.recurring[(callback_index as usize - 1) % self.recurring.len()]
        }
    }
}

fn append_engine_control_events(
    events: &mut Vec<TimedEvent>,
    frames: usize,
    reverse: bool,
) -> Result<(), BenchError> {
    let low =
        Normalized::new(if reverse { 1.0 } else { 0.0 }).map_err(|_| BenchError::InvalidConfig)?;
    let high =
        Normalized::new(if reverse { 0.0 } else { 1.0 }).map_err(|_| BenchError::InvalidConfig)?;
    for (offset, id, value) in [
        (0, MacroId::Shape, low),
        (frames / 4, MacroId::Color, high),
        (frames / 2, MacroId::Edge, low),
        (frames.saturating_mul(3) / 4, MacroId::Couple, high),
    ] {
        events.push(TimedEvent::new(
            offset.min(frames - 1),
            Event::SetMacro { id, value },
        ));
    }
    events.sort_unstable_by_key(|event| event.sample_offset);
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum CandidateEvent {
    NoteOn { note: u8, velocity: f32 },
    NoteOff { note: u8 },
}

#[derive(Clone, Copy, Debug)]
struct CandidateTimedEvent {
    sample_offset: usize,
    event: CandidateEvent,
}

#[derive(Clone, Debug)]
struct CandidateSchedule {
    initial: Box<[CandidateTimedEvent]>,
    recurring: Vec<Box<[CandidateTimedEvent]>>,
    maximum_offset: usize,
    maximum_events: usize,
}

impl CandidateSchedule {
    fn new(config: CaseConfig) -> Result<Self, BenchError> {
        if !config.is_applicable() {
            return Err(BenchError::UnsupportedCandidateControlScenario);
        }
        let initial = active_notes(config.scenario, config.voices)
            .into_iter()
            .map(|note| CandidateTimedEvent {
                sample_offset: 0,
                event: CandidateEvent::NoteOn {
                    note,
                    velocity: 0.82,
                },
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let mut recurring = Vec::with_capacity(EVENT_CYCLE_BLOCKS);
        for cycle in 0..EVENT_CYCLE_BLOCKS {
            let mut events = Vec::with_capacity(2);
            if config.scenario == Scenario::VoiceStealing && cycle % 4 == 0 {
                let note_index = (cycle / 4) % STEAL_NOTES.len();
                events.push(CandidateTimedEvent {
                    sample_offset: 0,
                    event: CandidateEvent::NoteOff {
                        note: STEAL_NOTES[note_index],
                    },
                });
                events.push(CandidateTimedEvent {
                    sample_offset: config.frames / 2,
                    event: CandidateEvent::NoteOn {
                        note: STEAL_NOTES[(note_index + config.voices + 1) % STEAL_NOTES.len()],
                        velocity: 0.78,
                    },
                });
            }
            recurring.push(events.into_boxed_slice());
        }
        let maximum_offset = initial
            .iter()
            .chain(recurring.iter().flat_map(|events| events.iter()))
            .map(|event| event.sample_offset)
            .max()
            .unwrap_or(0);
        let maximum_events = recurring
            .iter()
            .map(|events| events.len())
            .chain(std::iter::once(initial.len()))
            .max()
            .unwrap_or(0);
        if maximum_offset >= config.frames || maximum_events > MAX_EVENTS_PER_BLOCK {
            return Err(BenchError::InvalidConfig);
        }
        Ok(Self {
            initial,
            recurring,
            maximum_offset,
            maximum_events,
        })
    }

    fn events(&self, callback_index: u64) -> &[CandidateTimedEvent] {
        if callback_index == 0 {
            &self.initial
        } else {
            &self.recurring[(callback_index as usize - 1) % self.recurring.len()]
        }
    }
}

fn active_notes(scenario: Scenario, voices: usize) -> Vec<u8> {
    let count = if scenario == Scenario::HeldNote {
        1
    } else {
        voices
    };
    CHORD_NOTES[..count].to_vec()
}

#[derive(Debug)]
struct CandidateSlot {
    note: u8,
    age: u64,
    voice: ModelDVoice,
}

#[derive(Debug)]
struct CandidateBank {
    voices: Vec<CandidateSlot>,
    note_age: u64,
}

impl CandidateBank {
    fn new(sample_rate: u32, voices: usize) -> Result<Self, ModelDError> {
        let mut slots = Vec::with_capacity(voices);
        for _ in 0..voices {
            slots.push(CandidateSlot {
                note: 0,
                age: 0,
                voice: idealized_bass_voice(sample_rate as f32)?,
            });
        }
        Ok(Self {
            voices: slots,
            note_age: 0,
        })
    }

    fn apply_event(&mut self, event: CandidateEvent) {
        match event {
            CandidateEvent::NoteOn { note, velocity } if velocity > 0.0 => {
                self.note_age = self.note_age.wrapping_add(1);
                let index = self
                    .voices
                    .iter()
                    .position(|slot| slot.voice.is_idle())
                    .or_else(|| {
                        self.voices
                            .iter()
                            .enumerate()
                            .min_by_key(|(_, slot)| slot.age)
                            .map(|(index, _)| index)
                    });
                if let Some(index) = index {
                    let slot = &mut self.voices[index];
                    slot.note = note;
                    slot.age = self.note_age;
                    slot.voice.note_on(note, velocity);
                }
            }
            CandidateEvent::NoteOn { note, .. } | CandidateEvent::NoteOff { note } => {
                for slot in &mut self.voices {
                    if slot.note == note && !slot.voice.is_idle() {
                        slot.voice.note_off();
                    }
                }
            }
        }
    }

    fn active_voice_count(&self) -> usize {
        self.voices
            .iter()
            .filter(|slot| !slot.voice.is_idle())
            .count()
    }
}

#[derive(Debug)]
enum PreparedPath {
    Production {
        engine: Engine,
        schedule: EngineSchedule,
    },
    ModelD {
        bank: CandidateBank,
        schedule: CandidateSchedule,
    },
}

#[derive(Debug)]
pub struct PreparedCase {
    config: CaseConfig,
    path: PreparedPath,
    left: Vec<f32>,
    right: Vec<f32>,
    callback_index: u64,
}

impl PreparedCase {
    pub fn new(config: CaseConfig) -> Result<Self, BenchError> {
        if config.sample_rate == 0
            || config.frames == 0
            || config.voices == 0
            || config.voices > MAX_BENCH_VOICES
            || !config.engine_macros.validate()
        {
            return Err(BenchError::InvalidConfig);
        }
        let path = match config.path {
            RenderPath::ProductionEngine => {
                let mut preset = Preset::parse(include_str!("../presets/reference.mojsint"))?;
                preset.voices = config.voices;
                preset.macros.shape = normalized(config.engine_macros.shape)?;
                preset.macros.color = normalized(config.engine_macros.color)?;
                preset.macros.edge = normalized(config.engine_macros.edge)?;
                preset.macros.couple = normalized(config.engine_macros.couple)?;
                let preset = preset.validate()?;
                PreparedPath::Production {
                    engine: Engine::new(config.sample_rate as f32, &preset)?,
                    schedule: EngineSchedule::new(config)?,
                }
            }
            RenderPath::ModelDIdealized => PreparedPath::ModelD {
                bank: CandidateBank::new(config.sample_rate, config.voices)?,
                schedule: CandidateSchedule::new(config)?,
            },
        };
        Ok(Self {
            config,
            path,
            left: vec![0.0; config.frames],
            right: vec![0.0; config.frames],
            callback_index: 0,
        })
    }

    #[inline]
    pub fn render_callback(&mut self) -> Result<(), BenchError> {
        match &mut self.path {
            PreparedPath::Production { engine, schedule } => {
                let events = schedule.events(self.callback_index);
                engine.render_block(events, &mut self.left, &mut self.right)?;
            }
            PreparedPath::ModelD { bank, schedule } => {
                let events = schedule.events(self.callback_index);
                let mut event_index = 0;
                for sample_index in 0..self.left.len() {
                    while event_index < events.len()
                        && events[event_index].sample_offset == sample_index
                    {
                        bank.apply_event(events[event_index].event);
                        event_index += 1;
                    }
                    let mut mixed = 0.0_f32;
                    for slot in &mut bank.voices {
                        mixed += slot.voice.sample();
                    }
                    let sample = if mixed.is_finite() { mixed } else { 0.0 };
                    self.left[sample_index] = sample;
                    self.right[sample_index] = sample;
                }
            }
        }
        self.callback_index = self.callback_index.wrapping_add(1);
        Ok(())
    }

    pub fn output(&self) -> &[f32] {
        &self.left
    }

    pub fn right_output(&self) -> &[f32] {
        &self.right
    }

    pub fn active_voice_count(&self) -> usize {
        match &self.path {
            PreparedPath::Production { engine, .. } => engine.active_voice_count(),
            PreparedPath::ModelD { bank, .. } => bank.active_voice_count(),
        }
    }

    pub fn maximum_event_offset(&self) -> usize {
        match &self.path {
            PreparedPath::Production { schedule, .. } => schedule.maximum_offset,
            PreparedPath::ModelD { schedule, .. } => schedule.maximum_offset,
        }
    }

    pub fn maximum_events_per_block(&self) -> usize {
        match &self.path {
            PreparedPath::Production { schedule, .. } => schedule.maximum_events,
            PreparedPath::ModelD { schedule, .. } => schedule.maximum_events,
        }
    }

    pub const fn render_path_source(&self) -> &'static str {
        match self.config.path {
            RenderPath::ProductionEngine => "Engine::render_block",
            RenderPath::ModelDIdealized => "04_matched_idealized_path.wav",
        }
    }

    pub const fn config(&self) -> CaseConfig {
        self.config
    }
}

fn normalized(value: f32) -> Result<Normalized, BenchError> {
    Normalized::new(value).map_err(|_| BenchError::InvalidConfig)
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimingSummary {
    pub callback_count: usize,
    pub mean_ns: f64,
    pub median_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
    pub p999_ns: u64,
    pub maximum_ns: u64,
    pub mean_period_utilization_percent: f64,
    pub p999_period_utilization_percent: f64,
    pub maximum_period_utilization_percent: f64,
    pub deadline_misses: usize,
}

impl TimingSummary {
    pub fn from_durations(durations: &[u64], period_ns: u64) -> Result<Self, BenchError> {
        if durations.is_empty() || period_ns == 0 {
            return Err(BenchError::EmptyTiming);
        }
        let mut sorted = durations.to_vec();
        sorted.sort_unstable();
        let total = durations.iter().map(|value| *value as u128).sum::<u128>();
        let mean_ns = total as f64 / durations.len() as f64;
        let median_ns = percentile(&sorted, 50.0);
        let p95_ns = percentile(&sorted, 95.0);
        let p99_ns = percentile(&sorted, 99.0);
        let p999_ns = percentile(&sorted, 99.9);
        let maximum_ns = *sorted.last().expect("non-empty timing set");
        let utilization = |value: f64| value * 100.0 / period_ns as f64;
        Ok(Self {
            callback_count: durations.len(),
            mean_ns,
            median_ns,
            p95_ns,
            p99_ns,
            p999_ns,
            maximum_ns,
            mean_period_utilization_percent: utilization(mean_ns),
            p999_period_utilization_percent: utilization(p999_ns as f64),
            maximum_period_utilization_percent: utilization(maximum_ns as f64),
            deadline_misses: durations
                .iter()
                .filter(|duration| **duration > period_ns)
                .count(),
        })
    }
}

fn percentile(sorted: &[u64], percentage: f64) -> u64 {
    let rank = ((percentage / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformInfo {
    pub architecture: &'static str,
    pub kernel: String,
    pub cpu_model: String,
    pub label: String,
}

pub fn detect_platform() -> Result<PlatformInfo, BenchError> {
    let architecture = std::env::consts::ARCH;
    let kernel = Command::new("uname")
        .arg("-r")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())?;
    let cpuinfo = fs::read_to_string("/proc/cpuinfo")?;
    let cpu_model = cpuinfo
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            matches!(key.trim(), "Model" | "model name").then(|| value.trim().to_owned())
        })
        .unwrap_or_else(|| "unknown CPU".to_owned());
    Ok(PlatformInfo {
        architecture,
        kernel,
        cpu_model: cpu_model.clone(),
        label: format!("{architecture} {cpu_model}"),
    })
}
