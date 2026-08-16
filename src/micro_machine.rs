//! Strict, fixed-state typed micro-machine graph for the isolated swarm experiment.

use crate::envelope::{Adsr, AdsrConfig, EnvelopeError};
use serde::Deserialize;
use std::mem::size_of;
use thiserror::Error;

pub const MAX_NODES: usize = 16;
pub const MAX_EDGES: usize = 16;
pub const MAX_OSCILLATORS: usize = 12;
pub const MAX_STATE_BYTES: usize = 64 * 1024;
pub const MAX_WORK_UNITS_PER_SAMPLE: usize = 1_024;

const CONTROL_NAMES: [&str; 8] = [
    "MASS", "DETUNE", "SPREAD", "SHAPE", "BITE", "MOTION", "COLOR", "SPACE",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwarmControls {
    pub mass: f32,
    pub detune: f32,
    pub spread: f32,
    pub shape: f32,
    pub bite: f32,
    pub motion: f32,
    pub color: f32,
    pub space: f32,
}

impl SwarmControls {
    pub const NEUTRAL: Self = Self {
        mass: 0.52,
        detune: 0.42,
        spread: 0.58,
        shape: 0.30,
        bite: 0.26,
        motion: 0.34,
        color: 0.52,
        space: 0.48,
    };

    pub const LEAD: Self = Self {
        mass: 0.68,
        detune: 0.30,
        spread: 0.38,
        shape: 0.46,
        bite: 0.72,
        motion: 0.24,
        color: 0.76,
        space: 0.32,
    };

    pub const PAD: Self = Self {
        mass: 0.92,
        detune: 0.68,
        spread: 0.88,
        shape: 0.22,
        bite: 0.18,
        motion: 0.62,
        color: 0.38,
        space: 0.78,
    };

    pub const fn uniform(value: f32) -> Self {
        Self {
            mass: value,
            detune: value,
            spread: value,
            shape: value,
            bite: value,
            motion: value,
            color: value,
            space: value,
        }
    }

    pub fn with(mut self, index: usize, value: f32) -> Result<Self, MicroMachineError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(MicroMachineError::InvalidControls);
        }
        match index {
            0 => self.mass = value,
            1 => self.detune = value,
            2 => self.spread = value,
            3 => self.shape = value,
            4 => self.bite = value,
            5 => self.motion = value,
            6 => self.color = value,
            7 => self.space = value,
            _ => return Err(MicroMachineError::InvalidControls),
        }
        Ok(self)
    }

    fn values(self) -> [f32; 8] {
        [
            self.mass,
            self.detune,
            self.spread,
            self.shape,
            self.bite,
            self.motion,
            self.color,
            self.space,
        ]
    }

    fn validate(self) -> Result<Self, MicroMachineError> {
        if self
            .values()
            .into_iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(&value))
        {
            Ok(self)
        } else {
            Err(MicroMachineError::InvalidControls)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PortKind {
    PhaseBank,
    OscillatorBank,
    AudioStereo,
    GuardedStereo,
}

impl PortKind {
    const fn name(self) -> &'static str {
        match self {
            Self::PhaseBank => "PhaseBank",
            Self::OscillatorBank => "OscillatorBank",
            Self::AudioStereo => "AudioStereo",
            Self::GuardedStereo => "GuardedStereo",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeKind {
    PhaseSwarm,
    BandlimitedSawBank,
    BoundedStereoMix,
    SpectralTilt,
    BoundedDrive,
    StereoWidth,
    OutputGuard,
}

impl NodeKind {
    fn parse(value: &str) -> Result<Self, MicroMachineError> {
        match value {
            "phase_swarm" => Ok(Self::PhaseSwarm),
            "bandlimited_saw_bank" => Ok(Self::BandlimitedSawBank),
            "bounded_stereo_mix" => Ok(Self::BoundedStereoMix),
            "spectral_tilt" => Ok(Self::SpectralTilt),
            "bounded_drive" => Ok(Self::BoundedDrive),
            "stereo_width" => Ok(Self::StereoWidth),
            "output_guard" => Ok(Self::OutputGuard),
            _ => Err(MicroMachineError::UnknownNodeType(value.to_owned())),
        }
    }

    const fn input(self) -> Option<PortKind> {
        match self {
            Self::PhaseSwarm => None,
            Self::BandlimitedSawBank => Some(PortKind::PhaseBank),
            Self::BoundedStereoMix => Some(PortKind::OscillatorBank),
            Self::SpectralTilt | Self::BoundedDrive | Self::StereoWidth | Self::OutputGuard => {
                Some(PortKind::AudioStereo)
            }
        }
    }

    const fn output(self) -> PortKind {
        match self {
            Self::PhaseSwarm => PortKind::PhaseBank,
            Self::BandlimitedSawBank => PortKind::OscillatorBank,
            Self::BoundedStereoMix
            | Self::SpectralTilt
            | Self::BoundedDrive
            | Self::StereoWidth => PortKind::AudioStereo,
            Self::OutputGuard => PortKind::GuardedStereo,
        }
    }

    const fn output_name(self) -> &'static str {
        match self {
            Self::PhaseSwarm => "phases",
            Self::BandlimitedSawBank => "waves",
            Self::BoundedStereoMix
            | Self::SpectralTilt
            | Self::BoundedDrive
            | Self::StereoWidth => "stereo",
            Self::OutputGuard => "output",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GraphDocument {
    schema_version: u32,
    name: String,
    seed: u64,
    controls: Vec<String>,
    output: String,
    nodes: Vec<NodeDocument>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeDocument {
    id: String,
    #[serde(rename = "type")]
    node_type: String,
    input: Option<String>,
    max_oscillators: Option<usize>,
    detune_cents: Option<f32>,
    drift_cents: Option<f32>,
    coupling: Option<f32>,
    min_cutoff_hz: Option<f32>,
    max_cutoff_hz: Option<f32>,
    drive: Option<f32>,
    max_width: Option<f32>,
    ceiling: Option<f32>,
}

#[derive(Debug, Error)]
pub enum MicroMachineError {
    #[error("graph TOML is invalid: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("unsupported micro-machine schema version {0}")]
    UnsupportedVersion(u32),
    #[error("graph name must not be empty")]
    EmptyName,
    #[error("graph must expose exactly MASS, DETUNE, SPREAD, SHAPE, BITE, MOTION, COLOR, SPACE")]
    InvalidControlContract,
    #[error("unknown node type `{0}`")]
    UnknownNodeType(String),
    #[error("node `{0}` has an invalid or empty identifier")]
    InvalidNodeId(String),
    #[error("duplicate node identifier `{0}`")]
    DuplicateNode(String),
    #[error("node `{node}` is missing required field `{field}`")]
    MissingField { node: String, field: &'static str },
    #[error("node `{node}` does not allow field `{field}`")]
    UnexpectedField { node: String, field: &'static str },
    #[error("node `{node}` contains an invalid or non-finite value for `{field}`")]
    InvalidValue { node: String, field: &'static str },
    #[error("connection `{0}` must use `node.port` syntax")]
    InvalidConnection(String),
    #[error("connection references unknown node `{0}`")]
    UnknownConnectionNode(String),
    #[error("connection `{connection}` names an unknown output port")]
    UnknownOutputPort { connection: String },
    #[error("incompatible port for `{node}`: expected {expected}, received {actual}")]
    IncompatiblePort {
        node: String,
        expected: &'static str,
        actual: &'static str,
    },
    #[error("graph contains an invalid instantaneous cycle")]
    InvalidCycle,
    #[error("graph output must resolve to a GuardedStereo port")]
    InvalidOutput,
    #[error("resource limit violation: {0}")]
    ResourceLimit(&'static str),
    #[error("sample rate must be finite and between 8,000 and 384,000 Hz")]
    InvalidSampleRate,
    #[error("frequency must be finite, positive, and below Nyquist")]
    InvalidFrequency,
    #[error("all eight live controls must be finite normalized values")]
    InvalidControls,
    #[error("outer ADSR is invalid: {0}")]
    Envelope(#[from] EnvelopeError),
}

#[derive(Clone, Debug)]
pub struct MicroMachineGraph {
    document: GraphDocument,
}

impl MicroMachineGraph {
    pub fn parse(source: &str) -> Result<Self, MicroMachineError> {
        let document: GraphDocument = toml::from_str(source)?;
        if document.schema_version != 1 {
            return Err(MicroMachineError::UnsupportedVersion(
                document.schema_version,
            ));
        }
        if document.name.trim().is_empty() {
            return Err(MicroMachineError::EmptyName);
        }
        if document.controls.len() != CONTROL_NAMES.len()
            || !document
                .controls
                .iter()
                .zip(CONTROL_NAMES)
                .all(|(actual, expected)| actual == expected)
        {
            return Err(MicroMachineError::InvalidControlContract);
        }
        Ok(Self { document })
    }

    pub fn control_names(&self) -> &[String] {
        &self.document.controls
    }

    pub fn compile(
        &self,
        sample_rate: f32,
        frequency_hz: f32,
        controls: SwarmControls,
    ) -> Result<CompiledMicroMachine, MicroMachineError> {
        if !sample_rate.is_finite() || !(8_000.0..=384_000.0).contains(&sample_rate) {
            return Err(MicroMachineError::InvalidSampleRate);
        }
        if !frequency_hz.is_finite() || frequency_hz <= 0.0 || frequency_hz >= 0.5 * sample_rate {
            return Err(MicroMachineError::InvalidFrequency);
        }
        let controls = controls.validate()?;
        let node_count = self.document.nodes.len();
        if node_count == 0 || node_count > MAX_NODES {
            return Err(MicroMachineError::ResourceLimit("node count"));
        }

        let mut kinds = Vec::with_capacity(node_count);
        for (index, node) in self.document.nodes.iter().enumerate() {
            validate_identifier(&node.id)?;
            if self.document.nodes[..index]
                .iter()
                .any(|other| other.id == node.id)
            {
                return Err(MicroMachineError::DuplicateNode(node.id.clone()));
            }
            let kind = NodeKind::parse(&node.node_type)?;
            validate_node_fields(node, kind)?;
            kinds.push(kind);
        }

        let mut dependencies = vec![None; node_count];
        let mut edge_count = 0;
        for (index, node) in self.document.nodes.iter().enumerate() {
            match (kinds[index].input(), node.input.as_deref()) {
                (None, None) => {}
                (None, Some(_)) => {
                    return Err(MicroMachineError::UnexpectedField {
                        node: node.id.clone(),
                        field: "input",
                    });
                }
                (Some(_), None) => {
                    return Err(MicroMachineError::MissingField {
                        node: node.id.clone(),
                        field: "input",
                    });
                }
                (Some(expected), Some(connection)) => {
                    let source = resolve_connection(connection, &self.document.nodes, &kinds)?;
                    let actual = kinds[source].output();
                    if actual != expected {
                        return Err(MicroMachineError::IncompatiblePort {
                            node: node.id.clone(),
                            expected: expected.name(),
                            actual: actual.name(),
                        });
                    }
                    dependencies[index] = Some(source);
                    edge_count += 1;
                }
            }
        }
        if edge_count > MAX_EDGES {
            return Err(MicroMachineError::ResourceLimit("edge count"));
        }

        let output_original =
            resolve_connection(&self.document.output, &self.document.nodes, &kinds)?;
        if kinds[output_original].output() != PortKind::GuardedStereo {
            return Err(MicroMachineError::InvalidOutput);
        }

        let order = topological_order(&dependencies)?;
        let mut schedule_slot = vec![0; node_count];
        for (slot, original) in order.iter().copied().enumerate() {
            schedule_slot[original] = slot;
        }

        let mut plan = Vec::with_capacity(node_count);
        let mut state = Vec::with_capacity(node_count);
        let mut oscillator_slots = 0;
        let mut work_units = 0;
        for original in order {
            let node = &self.document.nodes[original];
            let kind = kinds[original];
            let input = dependencies[original].map(|source| schedule_slot[source]);
            let node_state =
                NodeState::prepare(node, kind, sample_rate, frequency_hz, self.document.seed);
            if let NodeState::Phase(phase) = &node_state {
                oscillator_slots += phase.count;
            }
            work_units += work_units_for(kind, node);
            plan.push(PlanNode {
                id: node.id.clone().into_boxed_str(),
                kind,
                input,
                config_hash: fingerprint_node(node, kind, input),
            });
            state.push(node_state);
        }
        let resource_usage = ResourceUsage {
            nodes: node_count,
            edges: edge_count,
            oscillator_slots,
            state_bytes: state.iter().map(NodeState::storage_bytes).sum::<usize>()
                + size_of::<[Signal; MAX_NODES]>()
                + size_of::<ControlState>()
                + size_of::<f32>(),
            work_units_per_sample: work_units,
        };
        if !resource_usage.within_limits() {
            return Err(MicroMachineError::ResourceLimit(
                "state bytes or per-sample work",
            ));
        }
        let output_slot = schedule_slot[output_original];
        let plan_fingerprint = fingerprint_plan(
            self.document.schema_version,
            self.document.seed,
            &plan,
            resource_usage,
            output_slot,
        );
        Ok(CompiledMicroMachine {
            plan: plan.into_boxed_slice(),
            state: state.into_boxed_slice(),
            signals: std::array::from_fn(|_| Signal::Empty),
            controls: ControlState::new(controls, sample_rate),
            output_slot,
            resource_usage,
            plan_fingerprint,
            maximum_internal_magnitude: 0.0,
        })
    }
}

fn validate_identifier(id: &str) -> Result<(), MicroMachineError> {
    let valid = !id.is_empty()
        && id.len() <= 48
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(MicroMachineError::InvalidNodeId(id.to_owned()))
    }
}

fn validate_node_fields(node: &NodeDocument, kind: NodeKind) -> Result<(), MicroMachineError> {
    let fields = [
        ("max_oscillators", node.max_oscillators.is_some()),
        ("detune_cents", node.detune_cents.is_some()),
        ("drift_cents", node.drift_cents.is_some()),
        ("coupling", node.coupling.is_some()),
        ("min_cutoff_hz", node.min_cutoff_hz.is_some()),
        ("max_cutoff_hz", node.max_cutoff_hz.is_some()),
        ("drive", node.drive.is_some()),
        ("max_width", node.max_width.is_some()),
        ("ceiling", node.ceiling.is_some()),
    ];
    let allowed: &[&str] = match kind {
        NodeKind::PhaseSwarm => &["max_oscillators", "detune_cents", "drift_cents", "coupling"],
        NodeKind::SpectralTilt => &["min_cutoff_hz", "max_cutoff_hz"],
        NodeKind::BoundedDrive => &["drive"],
        NodeKind::StereoWidth => &["max_width"],
        NodeKind::OutputGuard => &["ceiling"],
        NodeKind::BandlimitedSawBank | NodeKind::BoundedStereoMix => &[],
    };
    for (field, present) in fields {
        if present && !allowed.contains(&field) {
            return Err(MicroMachineError::UnexpectedField {
                node: node.id.clone(),
                field,
            });
        }
    }
    for field in allowed {
        if !fields
            .iter()
            .any(|(name, present)| name == field && *present)
        {
            return Err(MicroMachineError::MissingField {
                node: node.id.clone(),
                field,
            });
        }
    }
    if kind == NodeKind::PhaseSwarm {
        let count = node.max_oscillators.unwrap_or(0);
        if !(1..=MAX_OSCILLATORS).contains(&count) {
            return Err(MicroMachineError::ResourceLimit("oscillator slots"));
        }
        validate_range(node, "detune_cents", node.detune_cents, 0.0, 80.0)?;
        validate_range(node, "drift_cents", node.drift_cents, 0.0, 20.0)?;
        validate_range(node, "coupling", node.coupling, 0.0, 1.0)?;
    } else if kind == NodeKind::SpectralTilt {
        validate_range(node, "min_cutoff_hz", node.min_cutoff_hz, 40.0, 20_000.0)?;
        validate_range(node, "max_cutoff_hz", node.max_cutoff_hz, 100.0, 24_000.0)?;
        if node.min_cutoff_hz.unwrap_or(0.0) >= node.max_cutoff_hz.unwrap_or(0.0) {
            return Err(MicroMachineError::InvalidValue {
                node: node.id.clone(),
                field: "cutoff range",
            });
        }
    } else if kind == NodeKind::BoundedDrive {
        validate_range(node, "drive", node.drive, 0.0, 8.0)?;
    } else if kind == NodeKind::StereoWidth {
        validate_range(node, "max_width", node.max_width, 0.0, 2.0)?;
    } else if kind == NodeKind::OutputGuard {
        validate_range(node, "ceiling", node.ceiling, 0.1, 1.0)?;
    }
    Ok(())
}

fn validate_range(
    node: &NodeDocument,
    field: &'static str,
    value: Option<f32>,
    minimum: f32,
    maximum: f32,
) -> Result<(), MicroMachineError> {
    if value.is_some_and(|value| value.is_finite() && (minimum..=maximum).contains(&value)) {
        Ok(())
    } else {
        Err(MicroMachineError::InvalidValue {
            node: node.id.clone(),
            field,
        })
    }
}

fn resolve_connection(
    connection: &str,
    nodes: &[NodeDocument],
    kinds: &[NodeKind],
) -> Result<usize, MicroMachineError> {
    let Some((node_id, port)) = connection.split_once('.') else {
        return Err(MicroMachineError::InvalidConnection(connection.to_owned()));
    };
    if node_id.is_empty() || port.is_empty() || port.contains('.') {
        return Err(MicroMachineError::InvalidConnection(connection.to_owned()));
    }
    let Some(index) = nodes.iter().position(|node| node.id == node_id) else {
        return Err(MicroMachineError::UnknownConnectionNode(node_id.to_owned()));
    };
    if kinds[index].output_name() != port {
        return Err(MicroMachineError::UnknownOutputPort {
            connection: connection.to_owned(),
        });
    }
    Ok(index)
}

fn topological_order(dependencies: &[Option<usize>]) -> Result<Vec<usize>, MicroMachineError> {
    let mut selected = [false; MAX_NODES];
    let mut order = Vec::with_capacity(dependencies.len());
    for _ in 0..dependencies.len() {
        let next = dependencies
            .iter()
            .enumerate()
            .position(|(index, dependency)| {
                !selected[index] && dependency.is_none_or(|source| selected[source])
            });
        let Some(next) = next else {
            return Err(MicroMachineError::InvalidCycle);
        };
        selected[next] = true;
        order.push(next);
    }
    Ok(order)
}

fn work_units_for(kind: NodeKind, node: &NodeDocument) -> usize {
    match kind {
        NodeKind::PhaseSwarm => node.max_oscillators.unwrap_or(0) * 9,
        NodeKind::BandlimitedSawBank => MAX_OSCILLATORS * 12,
        NodeKind::BoundedStereoMix => MAX_OSCILLATORS * 7,
        NodeKind::SpectralTilt => 14,
        NodeKind::BoundedDrive => 12,
        NodeKind::StereoWidth => 8,
        NodeKind::OutputGuard => 12,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResourceUsage {
    pub nodes: usize,
    pub edges: usize,
    pub oscillator_slots: usize,
    pub state_bytes: usize,
    pub work_units_per_sample: usize,
}

impl ResourceUsage {
    pub const fn within_limits(self) -> bool {
        self.nodes <= MAX_NODES
            && self.edges <= MAX_EDGES
            && self.oscillator_slots <= MAX_OSCILLATORS
            && self.state_bytes <= MAX_STATE_BYTES
            && self.work_units_per_sample <= MAX_WORK_UNITS_PER_SAMPLE
    }
}

#[derive(Debug)]
struct PlanNode {
    id: Box<str>,
    kind: NodeKind,
    input: Option<usize>,
    config_hash: u64,
}

#[derive(Clone, Copy, Debug)]
struct PhaseFrame {
    count: usize,
    phase: [f32; MAX_OSCILLATORS],
    increment: [f32; MAX_OSCILLATORS],
    pan: [f32; MAX_OSCILLATORS],
}

impl Default for PhaseFrame {
    fn default() -> Self {
        Self {
            count: 0,
            phase: [0.0; MAX_OSCILLATORS],
            increment: [0.0; MAX_OSCILLATORS],
            pan: [0.0; MAX_OSCILLATORS],
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct OscillatorFrame {
    count: usize,
    values: [f32; MAX_OSCILLATORS],
    pan: [f32; MAX_OSCILLATORS],
}

impl Default for OscillatorFrame {
    fn default() -> Self {
        Self {
            count: 0,
            values: [0.0; MAX_OSCILLATORS],
            pan: [0.0; MAX_OSCILLATORS],
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Signal {
    Empty,
    Phases(PhaseFrame),
    Oscillators(OscillatorFrame),
    Stereo([f32; 2]),
    Guarded([f32; 2]),
}

impl Signal {
    #[inline]
    fn magnitude(self) -> f32 {
        match self {
            Self::Empty => 0.0,
            Self::Phases(frame) => frame.phase[..frame.count]
                .iter()
                .chain(&frame.increment[..frame.count])
                .copied()
                .map(f32::abs)
                .fold(0.0, f32::max),
            Self::Oscillators(frame) => frame.values[..frame.count]
                .iter()
                .copied()
                .map(f32::abs)
                .fold(0.0, f32::max),
            Self::Stereo(frame) | Self::Guarded(frame) => frame[0].abs().max(frame[1].abs()),
        }
    }
}

#[derive(Debug)]
enum NodeState {
    Phase(Box<PhaseState>),
    SawBank,
    Mix,
    Tone(ToneState),
    Drive(DriveState),
    Width(WidthState),
    Guard(GuardState),
}

impl NodeState {
    fn prepare(
        node: &NodeDocument,
        kind: NodeKind,
        sample_rate: f32,
        frequency_hz: f32,
        seed: u64,
    ) -> Self {
        match kind {
            NodeKind::PhaseSwarm => Self::Phase(Box::new(PhaseState::new(
                sample_rate,
                frequency_hz,
                node.max_oscillators.unwrap_or(1),
                node.detune_cents.unwrap_or(0.0),
                node.drift_cents.unwrap_or(0.0),
                node.coupling.unwrap_or(0.0),
                seed,
            ))),
            NodeKind::BandlimitedSawBank => Self::SawBank,
            NodeKind::BoundedStereoMix => Self::Mix,
            NodeKind::SpectralTilt => Self::Tone(ToneState::new(
                sample_rate,
                node.min_cutoff_hz.unwrap_or(900.0),
                node.max_cutoff_hz.unwrap_or(18_000.0),
            )),
            NodeKind::BoundedDrive => Self::Drive(DriveState {
                maximum_drive: node.drive.unwrap_or(0.0),
            }),
            NodeKind::StereoWidth => Self::Width(WidthState {
                maximum_width: node.max_width.unwrap_or(1.0),
            }),
            NodeKind::OutputGuard => {
                Self::Guard(GuardState::new(sample_rate, node.ceiling.unwrap_or(0.92)))
            }
        }
    }

    #[inline]
    fn process(&mut self, input: Signal, controls: SwarmControls) -> Signal {
        match self {
            Self::Phase(state) => Signal::Phases(state.sample(controls)),
            Self::SawBank => match input {
                Signal::Phases(frame) => {
                    Signal::Oscillators(sample_saw_bank(frame, controls.shape))
                }
                _ => Signal::Oscillators(OscillatorFrame::default()),
            },
            Self::Mix => match input {
                Signal::Oscillators(frame) => Signal::Stereo(mix_swarm(frame, controls)),
                _ => Signal::Stereo([0.0; 2]),
            },
            Self::Tone(state) => match input {
                Signal::Stereo(frame) => Signal::Stereo(state.sample(frame, controls.color)),
                _ => Signal::Stereo([0.0; 2]),
            },
            Self::Drive(state) => match input {
                Signal::Stereo(frame) => Signal::Stereo(state.sample(frame, controls.bite)),
                _ => Signal::Stereo([0.0; 2]),
            },
            Self::Width(state) => match input {
                Signal::Stereo(frame) => Signal::Stereo(state.sample(frame, controls.space)),
                _ => Signal::Stereo([0.0; 2]),
            },
            Self::Guard(state) => match input {
                Signal::Stereo(frame) => Signal::Guarded(state.sample(frame)),
                _ => Signal::Guarded([0.0; 2]),
            },
        }
    }

    fn reset(&mut self) {
        match self {
            Self::Phase(state) => state.reset(),
            Self::Tone(state) => state.reset(),
            Self::Guard(state) => state.reset(),
            Self::SawBank | Self::Mix | Self::Drive(_) | Self::Width(_) => {}
        }
    }

    fn set_frequency(&mut self, frequency_hz: f32) {
        if let Self::Phase(state) = self {
            state.frequency_hz = frequency_hz;
        }
    }

    fn storage_bytes(&self) -> usize {
        size_of::<Self>()
            + if matches!(self, Self::Phase(_)) {
                size_of::<PhaseState>()
            } else {
                0
            }
    }
}

#[derive(Debug)]
struct PhaseState {
    sample_rate: f32,
    frequency_hz: f32,
    count: usize,
    detune_cents: f32,
    drift_cents: f32,
    coupling: f32,
    phase: [f32; MAX_OSCILLATORS],
    initial_phase: [f32; MAX_OSCILLATORS],
    drift_sine: [f32; MAX_OSCILLATORS],
    drift_cosine: [f32; MAX_OSCILLATORS],
    initial_drift_sine: [f32; MAX_OSCILLATORS],
    initial_drift_cosine: [f32; MAX_OSCILLATORS],
    rotation_sine: [f32; MAX_OSCILLATORS],
    rotation_cosine: [f32; MAX_OSCILLATORS],
    gear: [u32; MAX_OSCILLATORS],
    initial_gear: [u32; MAX_OSCILLATORS],
    detune_position: [f32; MAX_OSCILLATORS],
    samples_since_normalize: u16,
}

impl PhaseState {
    fn new(
        sample_rate: f32,
        frequency_hz: f32,
        count: usize,
        detune_cents: f32,
        drift_cents: f32,
        coupling: f32,
        seed: u64,
    ) -> Self {
        let mut state = Self {
            sample_rate,
            frequency_hz,
            count,
            detune_cents,
            drift_cents,
            coupling,
            phase: [0.0; MAX_OSCILLATORS],
            initial_phase: [0.0; MAX_OSCILLATORS],
            drift_sine: [0.0; MAX_OSCILLATORS],
            drift_cosine: [1.0; MAX_OSCILLATORS],
            initial_drift_sine: [0.0; MAX_OSCILLATORS],
            initial_drift_cosine: [1.0; MAX_OSCILLATORS],
            rotation_sine: [0.0; MAX_OSCILLATORS],
            rotation_cosine: [1.0; MAX_OSCILLATORS],
            gear: [0; MAX_OSCILLATORS],
            initial_gear: [0; MAX_OSCILLATORS],
            detune_position: [0.0; MAX_OSCILLATORS],
            samples_since_normalize: 0,
        };
        let mut random = (seed as u32) ^ ((seed >> 32) as u32) ^ 0xa511_e9b3;
        for index in 0..count {
            random = xorshift(random.wrapping_add((index as u32 + 1).wrapping_mul(0x9e37_79b9)));
            let phase = unit_from_u32(random);
            random = xorshift(random);
            let drift_phase = unit_from_u32(random) * std::f32::consts::TAU;
            let (drift_sine, drift_cosine) = drift_phase.sin_cos();
            let rate_hz = 0.071 + 0.019 * index as f32 + 0.007 * f32::from((random & 3) as u8);
            let (rotation_sine, rotation_cosine) =
                (std::f32::consts::TAU * rate_hz / sample_rate).sin_cos();
            let position = if count == 1 {
                0.0
            } else {
                2.0 * index as f32 / (count - 1) as f32 - 1.0
            };
            let curved = position.signum() * position.abs().powi(2);
            state.phase[index] = phase;
            state.initial_phase[index] = phase;
            state.drift_sine[index] = drift_sine;
            state.drift_cosine[index] = drift_cosine;
            state.initial_drift_sine[index] = drift_sine;
            state.initial_drift_cosine[index] = drift_cosine;
            state.rotation_sine[index] = rotation_sine;
            state.rotation_cosine[index] = rotation_cosine;
            state.gear[index] = random | 1;
            state.initial_gear[index] = random | 1;
            state.detune_position[index] = curved;
        }
        state
    }

    #[inline]
    fn sample(&mut self, controls: SwarmControls) -> PhaseFrame {
        let mut frame = PhaseFrame {
            count: self.count,
            ..PhaseFrame::default()
        };
        let detune_scale = 0.08 + 0.92 * controls.detune * controls.detune;
        let drift_scale = controls.motion * controls.motion;
        for index in 0..self.count {
            let sine = self.drift_sine[index];
            let cosine = self.drift_cosine[index];
            self.drift_sine[index] =
                sine * self.rotation_cosine[index] + cosine * self.rotation_sine[index];
            self.drift_cosine[index] =
                cosine * self.rotation_cosine[index] - sine * self.rotation_sine[index];

            let gear_bipolar = unit_from_u32(self.gear[index]) * 2.0 - 1.0;
            let cents = self.detune_position[index] * self.detune_cents * detune_scale
                + self.drift_cents * drift_scale * self.drift_sine[index]
                + 12.0 * self.coupling * controls.motion * gear_bipolar;
            let exponent = cents * (std::f32::consts::LN_2 / 1_200.0);
            let ratio = 1.0 + exponent + 0.5 * exponent * exponent;
            let increment = (self.frequency_hz * ratio / self.sample_rate).clamp(0.0, 0.49);
            frame.phase[index] = self.phase[index];
            frame.increment[index] = increment;
            frame.pan[index] = self.detune_position[index];
            self.phase[index] += increment;
            if self.phase[index] >= 1.0 {
                self.phase[index] -= 1.0;
                self.gear[index] = xorshift(
                    self.gear[index]
                        .rotate_left(3 + (index as u32 & 7))
                        .wrapping_add(0x6d2b_79f5),
                );
            }
        }
        self.samples_since_normalize += 1;
        if self.samples_since_normalize == 1_024 {
            for index in 0..self.count {
                let scale = (self.drift_sine[index] * self.drift_sine[index]
                    + self.drift_cosine[index] * self.drift_cosine[index])
                    .sqrt()
                    .recip();
                self.drift_sine[index] *= scale;
                self.drift_cosine[index] *= scale;
            }
            self.samples_since_normalize = 0;
        }
        frame
    }

    fn reset(&mut self) {
        self.phase = self.initial_phase;
        self.drift_sine = self.initial_drift_sine;
        self.drift_cosine = self.initial_drift_cosine;
        self.gear = self.initial_gear;
        self.samples_since_normalize = 0;
    }
}

#[inline]
fn sample_saw_bank(phases: PhaseFrame, shape: f32) -> OscillatorFrame {
    let mut output = OscillatorFrame {
        count: phases.count,
        pan: phases.pan,
        ..OscillatorFrame::default()
    };
    let pulse_mix = 0.72 * shape * shape;
    let width = 0.50 - 0.30 * shape;
    for index in 0..phases.count {
        let phase = phases.phase[index];
        let increment = phases.increment[index].max(1.0e-7);
        let saw = 2.0 * phase - 1.0 - poly_blep(phase, increment);
        let shifted = wrap_phase(phase - width);
        let pulse = if phase < width { 1.0 } else { -1.0 } + poly_blep(phase, increment)
            - poly_blep(shifted, increment);
        output.values[index] = finite_or_zero(saw + pulse_mix * (0.72 * pulse - saw));
    }
    output
}

#[inline]
fn mix_swarm(frame: OscillatorFrame, controls: SwarmControls) -> [f32; 2] {
    let population = 1.0 + controls.mass * (frame.count.saturating_sub(1)) as f32;
    let full = population.floor() as usize;
    let partial = population - full as f32;
    let mut left = 0.0;
    let mut right = 0.0;
    let mut energy = 0.0;
    for index in 0..frame.count {
        let weight = if index < full {
            1.0
        } else if index == full {
            partial
        } else {
            0.0
        };
        let pan = (frame.pan[index] * controls.spread).clamp(-1.0, 1.0);
        left += weight * frame.values[index] * 0.5 * (1.0 - pan);
        right += weight * frame.values[index] * 0.5 * (1.0 + pan);
        energy += weight * weight;
    }
    let normalization = 0.82 / energy.max(1.0).sqrt();
    [
        finite_or_zero(left * normalization),
        finite_or_zero(right * normalization),
    ]
}

#[derive(Debug)]
struct ToneState {
    low_coefficient: f32,
    high_coefficient: f32,
    value: [f32; 2],
}

impl ToneState {
    fn new(sample_rate: f32, minimum_hz: f32, maximum_hz: f32) -> Self {
        Self {
            low_coefficient: one_pole_coefficient(sample_rate, minimum_hz),
            high_coefficient: one_pole_coefficient(sample_rate, maximum_hz.min(0.45 * sample_rate)),
            value: [0.0; 2],
        }
    }

    #[inline]
    fn sample(&mut self, input: [f32; 2], color: f32) -> [f32; 2] {
        let curve = color * color;
        let coefficient =
            self.low_coefficient + curve * (self.high_coefficient - self.low_coefficient);
        for (channel, sample) in input.into_iter().enumerate() {
            self.value[channel] += coefficient * (sample - self.value[channel]);
        }
        self.value
    }

    fn reset(&mut self) {
        self.value = [0.0; 2];
    }
}

#[derive(Debug)]
struct DriveState {
    maximum_drive: f32,
}

impl DriveState {
    #[inline]
    fn sample(&self, input: [f32; 2], bite: f32) -> [f32; 2] {
        let drive = 1.0 + self.maximum_drive * bite * bite;
        let compensation = 1.0 / (1.0 + 0.20 * self.maximum_drive * bite);
        input.map(|sample| {
            let driven = (sample * drive).clamp(-2.0, 2.0);
            finite_or_zero(compensation * driven / (1.0 + 0.42 * driven * driven))
        })
    }
}

#[derive(Debug)]
struct WidthState {
    maximum_width: f32,
}

impl WidthState {
    #[inline]
    fn sample(&self, input: [f32; 2], space: f32) -> [f32; 2] {
        let mid = 0.5 * (input[0] + input[1]);
        let side = 0.5 * (input[0] - input[1]);
        let width = 0.15 + space * self.maximum_width;
        [mid + width * side, mid - width * side]
    }
}

#[derive(Debug)]
struct GuardState {
    ceiling: f32,
    dc_coefficient: f32,
    previous_input: [f32; 2],
    previous_output: [f32; 2],
}

impl GuardState {
    fn new(sample_rate: f32, ceiling: f32) -> Self {
        Self {
            ceiling,
            dc_coefficient: (-std::f32::consts::TAU * 7.0 / sample_rate).exp(),
            previous_input: [0.0; 2],
            previous_output: [0.0; 2],
        }
    }

    #[inline]
    fn sample(&mut self, input: [f32; 2]) -> [f32; 2] {
        let mut output = [0.0; 2];
        for (channel, sample) in input.into_iter().enumerate() {
            let guarded = finite_or_zero(sample);
            let dc = guarded - self.previous_input[channel]
                + self.dc_coefficient * self.previous_output[channel];
            self.previous_input[channel] = guarded;
            self.previous_output[channel] = finite_or_zero(dc);
            output[channel] = self.previous_output[channel].clamp(-self.ceiling, self.ceiling);
        }
        output
    }

    fn reset(&mut self) {
        self.previous_input = [0.0; 2];
        self.previous_output = [0.0; 2];
    }
}

#[derive(Debug)]
struct ControlState {
    current: SwarmControls,
    target: SwarmControls,
    maximum_step: f32,
}

impl ControlState {
    fn new(controls: SwarmControls, sample_rate: f32) -> Self {
        Self {
            current: controls,
            target: controls,
            maximum_step: 1.0 / (0.010 * sample_rate).max(1.0),
        }
    }

    #[inline]
    fn advance(&mut self) -> SwarmControls {
        let mut values = self.current.values();
        let targets = self.target.values();
        for index in 0..values.len() {
            let difference = targets[index] - values[index];
            values[index] += difference.clamp(-self.maximum_step, self.maximum_step);
        }
        self.current = SwarmControls {
            mass: values[0],
            detune: values[1],
            spread: values[2],
            shape: values[3],
            bite: values[4],
            motion: values[5],
            color: values[6],
            space: values[7],
        };
        self.current
    }

    fn reset(&mut self) {
        self.current = self.target;
    }
}

#[derive(Debug)]
pub struct CompiledMicroMachine {
    plan: Box<[PlanNode]>,
    state: Box<[NodeState]>,
    signals: [Signal; MAX_NODES],
    controls: ControlState,
    output_slot: usize,
    resource_usage: ResourceUsage,
    plan_fingerprint: u64,
    maximum_internal_magnitude: f32,
}

impl CompiledMicroMachine {
    #[inline]
    pub fn sample(&mut self) -> [f32; 2] {
        let controls = self.controls.advance();
        for index in 0..self.plan.len() {
            let input = self.plan[index]
                .input
                .map_or(Signal::Empty, |source| self.signals[source]);
            self.signals[index] = self.state[index].process(input, controls);
            self.maximum_internal_magnitude = self
                .maximum_internal_magnitude
                .max(self.signals[index].magnitude());
        }
        match self.signals[self.output_slot] {
            Signal::Guarded(frame) => frame,
            _ => [0.0; 2],
        }
    }

    pub fn render_block(&mut self, output: &mut [[f32; 2]]) {
        for frame in output {
            *frame = self.sample();
        }
    }

    pub fn set_controls(&mut self, controls: SwarmControls) -> Result<(), MicroMachineError> {
        self.controls.target = controls.validate()?;
        Ok(())
    }

    pub fn set_frequency(&mut self, frequency_hz: f32) -> Result<(), MicroMachineError> {
        let sample_rate = self.state.iter().find_map(|state| match state {
            NodeState::Phase(state) => Some(state.sample_rate),
            _ => None,
        });
        let Some(sample_rate) = sample_rate else {
            return Err(MicroMachineError::InvalidFrequency);
        };
        if !frequency_hz.is_finite() || frequency_hz <= 0.0 || frequency_hz >= 0.5 * sample_rate {
            return Err(MicroMachineError::InvalidFrequency);
        }
        for state in &mut self.state {
            state.set_frequency(frequency_hz);
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        for state in &mut self.state {
            state.reset();
        }
        self.signals.fill(Signal::Empty);
        self.controls.reset();
        self.maximum_internal_magnitude = 0.0;
    }

    pub const fn plan_fingerprint(&self) -> u64 {
        self.plan_fingerprint
    }

    pub fn scheduled_node_ids(&self) -> impl Iterator<Item = &str> {
        self.plan.iter().map(|node| node.id.as_ref())
    }

    pub const fn resource_usage(&self) -> ResourceUsage {
        self.resource_usage
    }

    pub fn state_address(&self) -> usize {
        self.state.as_ptr() as usize
    }

    pub const fn maximum_internal_magnitude(&self) -> f32 {
        self.maximum_internal_magnitude
    }
}

#[derive(Debug)]
pub struct MicroMachineVoice {
    machine: CompiledMicroMachine,
    envelope: Adsr,
    velocity: f32,
}

impl MicroMachineVoice {
    pub fn compile(
        graph: &MicroMachineGraph,
        sample_rate: f32,
        frequency_hz: f32,
        controls: SwarmControls,
        adsr: AdsrConfig,
    ) -> Result<Self, MicroMachineError> {
        Ok(Self {
            machine: graph.compile(sample_rate, frequency_hz, controls)?,
            envelope: Adsr::new(sample_rate, adsr)?,
            velocity: 0.0,
        })
    }

    pub fn note_on(&mut self, velocity: f32) {
        self.velocity = if velocity.is_finite() {
            velocity.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.machine.reset();
        self.envelope.restart();
    }

    pub fn note_off(&mut self) {
        self.envelope.note_off();
    }

    pub fn set_controls(&mut self, controls: SwarmControls) -> Result<(), MicroMachineError> {
        self.machine.set_controls(controls)
    }

    pub fn set_adsr(&mut self, adsr: AdsrConfig) {
        self.envelope.set_config(adsr);
    }

    #[inline]
    pub fn sample(&mut self) -> [f32; 2] {
        if self.envelope.is_idle() {
            return [0.0; 2];
        }
        let gain = self.envelope.advance() * self.velocity;
        let frame = self.machine.sample();
        let output = [
            finite_or_zero(frame[0] * gain),
            finite_or_zero(frame[1] * gain),
        ];
        if self.envelope.is_idle() {
            self.machine.reset();
        }
        output
    }

    pub fn reset(&mut self) {
        self.envelope.reset();
        self.machine.reset();
        self.velocity = 0.0;
    }

    pub fn panic(&mut self) {
        self.reset();
    }

    pub fn is_idle(&self) -> bool {
        self.envelope.is_idle()
    }
}

fn fingerprint_node(node: &NodeDocument, kind: NodeKind, input: Option<usize>) -> u64 {
    let values = [
        kind as u64,
        input.map_or(u64::MAX, |value| value as u64),
        node.max_oscillators.map_or(u64::MAX, |value| value as u64),
        node.detune_cents
            .map_or(u64::MAX, |value| u64::from(value.to_bits())),
        node.drift_cents
            .map_or(u64::MAX, |value| u64::from(value.to_bits())),
        node.coupling
            .map_or(u64::MAX, |value| u64::from(value.to_bits())),
        node.min_cutoff_hz
            .map_or(u64::MAX, |value| u64::from(value.to_bits())),
        node.max_cutoff_hz
            .map_or(u64::MAX, |value| u64::from(value.to_bits())),
        node.drive
            .map_or(u64::MAX, |value| u64::from(value.to_bits())),
        node.max_width
            .map_or(u64::MAX, |value| u64::from(value.to_bits())),
        node.ceiling
            .map_or(u64::MAX, |value| u64::from(value.to_bits())),
    ];
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in values.into_iter().flat_map(u64::to_le_bytes) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn fingerprint_plan(
    version: u32,
    seed: u64,
    plan: &[PlanNode],
    usage: ResourceUsage,
    output_slot: usize,
) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in version
        .to_le_bytes()
        .into_iter()
        .chain(seed.to_le_bytes())
        .chain((usage.work_units_per_sample as u64).to_le_bytes())
        .chain((output_slot as u64).to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    for node in plan {
        for byte in node
            .id
            .bytes()
            .chain([node.kind as u8])
            .chain(node.config_hash.to_le_bytes())
        {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

#[inline]
fn poly_blep(phase: f32, increment: f32) -> f32 {
    if phase < increment {
        let normalized = phase / increment;
        normalized + normalized - normalized * normalized - 1.0
    } else if phase > 1.0 - increment {
        let normalized = (phase - 1.0) / increment;
        normalized * normalized + normalized + normalized + 1.0
    } else {
        0.0
    }
}

#[inline]
fn wrap_phase(phase: f32) -> f32 {
    if phase < 0.0 { phase + 1.0 } else { phase }
}

fn one_pole_coefficient(sample_rate: f32, cutoff_hz: f32) -> f32 {
    1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate).exp()
}

#[inline]
fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

#[inline]
fn xorshift(mut value: u32) -> u32 {
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    value
}

#[inline]
fn unit_from_u32(value: u32) -> f32 {
    (value >> 8) as f32 * (1.0 / 16_777_216.0)
}
