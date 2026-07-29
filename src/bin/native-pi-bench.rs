use moj_sint::native_bench::{
    CaseConfig, EngineMacroConfig, PreparedCase, RenderPath, Scenario, TimingSummary,
    detect_platform,
};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::hint::black_box;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SAMPLE_RATE: u32 = 48_000;
const VOICE_COUNTS: [usize; 4] = [1, 2, 4, 8];
const DEFAULT_FRAMES: [usize; 2] = [64, 128];
const MAX_RSS_GROWTH_KIB: i64 = 1_024;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Clone, Copy, Debug)]
struct RunSettings {
    trial_seconds: u64,
    trials: usize,
    sustained_seconds: u64,
    warmup_audio_seconds: u64,
    paced: bool,
    callback_override: Option<usize>,
    smoke: bool,
}

#[derive(Clone, Debug, Default)]
struct SystemSnapshot {
    epoch_ms: u128,
    rss_kib: i64,
    hwm_kib: i64,
    voluntary_context_switches: u64,
    involuntary_context_switches: u64,
    migrations: u64,
    temperature_millicelsius: i64,
    frequency_khz: i64,
    throttled: String,
    affinity: String,
}

impl SystemSnapshot {
    fn capture() -> Self {
        let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
        let sched = fs::read_to_string("/proc/self/sched").unwrap_or_default();
        Self {
            epoch_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            rss_kib: status_value(&status, "VmRSS:"),
            hwm_kib: status_value(&status, "VmHWM:"),
            voluntary_context_switches: status_value(&status, "voluntary_ctxt_switches:").max(0)
                as u64,
            involuntary_context_switches: status_value(&status, "nonvoluntary_ctxt_switches:")
                .max(0) as u64,
            migrations: sched
                .lines()
                .find(|line| line.contains("nr_migrations"))
                .and_then(parse_colon_u64)
                .unwrap_or(0),
            temperature_millicelsius: read_i64("/sys/class/thermal/thermal_zone0/temp"),
            frequency_khz: read_i64("/sys/devices/system/cpu/cpufreq/policy0/scaling_cur_freq"),
            throttled: command_output("vcgencmd", &["get_throttled"])
                .unwrap_or_else(|| "unavailable".to_owned()),
            affinity: status
                .lines()
                .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))
                .map(str::trim)
                .unwrap_or("unknown")
                .to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
struct TrialRecord {
    config: CaseConfig,
    trial: usize,
    sustained: bool,
    requested_seconds: u64,
    elapsed_seconds: f64,
    period_ns: u64,
    timing: TimingSummary,
    scheduled_late_count: usize,
    maximum_start_lateness_ns: u64,
    output_peak: f32,
    non_finite_count: u64,
    output_hash: u64,
    active_voice_count: usize,
    expected_active_voice_count: usize,
    output_bound: f32,
    start: SystemSnapshot,
    end: SystemSnapshot,
    deterministic: bool,
    raw_path: PathBuf,
}

#[derive(Clone, Debug)]
struct SafetyRow {
    path: RenderPath,
    matrix_cap: Option<usize>,
    sustained_tested: Vec<usize>,
    final_cap: Option<usize>,
    boundary: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SoakRequest {
    path: RenderPath,
    scenario: Scenario,
    voices: usize,
    frames: usize,
}

impl TrialRecord {
    fn callback_pass(&self) -> bool {
        self.timing.deadline_misses == 0
            && self.non_finite_count == 0
            && self.output_peak <= self.output_bound
            && self.active_voice_count == self.expected_active_voice_count
            && self.timing.p999_period_utilization_percent <= 25.0
            && self.timing.maximum_period_utilization_percent <= 50.0
            && self.memory_stable()
            && self.deterministic
    }

    fn memory_stable(&self) -> bool {
        self.end.rss_kib - self.start.rss_kib <= MAX_RSS_GROWTH_KIB
    }

    fn no_new_throttling(&self) -> bool {
        self.start.throttled == self.end.throttled
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("native-pi-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut arguments = std::env::args().skip(1);
    let command = arguments.next().ok_or_else(usage)?;
    let output = PathBuf::from(arguments.next().ok_or_else(usage)?);
    let remaining = arguments.collect::<Vec<_>>();
    if command == "soak" {
        return run_soak(&output, parse_soak_request(remaining)?);
    }
    let settings = match command.as_str() {
        "suite" => parse_suite_settings(remaining)?,
        "smoke" if remaining.is_empty() => RunSettings {
            trial_seconds: 0,
            trials: 1,
            sustained_seconds: 0,
            warmup_audio_seconds: 0,
            paced: false,
            callback_override: Some(128),
            smoke: true,
        },
        _ => return Err(usage()),
    };
    prepare_output(&output)?;
    write_platform(&output)?;

    let prescan_callbacks = if settings.smoke { 64 } else { 4_096 };
    let worst_macros = run_engine_prescan(&output, prescan_callbacks)?;
    write_model_d_prescan(&output)?;
    let frames = callback_frames();

    let mut records = Vec::new();
    let paths: &[RenderPath] = if settings.smoke {
        &[RenderPath::ProductionEngine]
    } else {
        &RenderPath::ALL
    };
    let voices: &[usize] = if settings.smoke { &[1] } else { &VOICE_COUNTS };
    let scenarios: &[Scenario] = if settings.smoke {
        &[Scenario::HeldNote]
    } else {
        &Scenario::ALL
    };
    let selected_frames: Vec<usize> = if settings.smoke { vec![64] } else { frames };

    for &path in paths {
        for &period_frames in &selected_frames {
            for &voice_count in voices {
                for &scenario in scenarios {
                    let config = CaseConfig {
                        path,
                        scenario,
                        sample_rate: SAMPLE_RATE,
                        frames: period_frames,
                        voices: voice_count,
                        engine_macros: worst_macros,
                    };
                    if !config.is_applicable() {
                        continue;
                    }
                    for trial in 1..=settings.trials {
                        let record = run_trial(&output, config, trial, false, settings)?;
                        println!(
                            "trial path={} scenario={} voices={} frames={} trial={} p99.9={:.3}% max={:.3}% misses={} peak={:.6}",
                            path.as_str(),
                            scenario.as_str(),
                            voice_count,
                            period_frames,
                            trial,
                            record.timing.p999_period_utilization_percent,
                            record.timing.maximum_period_utilization_percent,
                            record.timing.deadline_misses,
                            record.output_peak
                        );
                        records.push(record);
                    }
                }
            }
        }
    }
    mark_determinism(&mut records);

    let mut safety_rows = Vec::new();
    if !settings.smoke && settings.sustained_seconds > 0 {
        for path in RenderPath::ALL {
            let matrix_cap = largest_matrix_cap(&records, path, settings.trials);
            let mut final_cap = None;
            let mut sustained_tested = Vec::new();
            if let Some(cap) = matrix_cap {
                for voice_count in VOICE_COUNTS.into_iter().rev().filter(|count| *count <= cap) {
                    if !voice_matrix_passes(&records, path, voice_count, settings.trials) {
                        continue;
                    }
                    let worst = worst_case(&records, path, voice_count)
                        .ok_or_else(|| "missing worst matrix case".to_owned())?;
                    let sustained_settings = RunSettings {
                        trial_seconds: settings.sustained_seconds,
                        trials: 1,
                        sustained_seconds: 0,
                        warmup_audio_seconds: settings.warmup_audio_seconds,
                        paced: true,
                        callback_override: None,
                        smoke: false,
                    };
                    let sustained_trial = settings.trials + sustained_tested.len() + 1;
                    let record = run_trial(
                        &output,
                        CaseConfig {
                            voices: voice_count,
                            ..worst.config
                        },
                        sustained_trial,
                        true,
                        sustained_settings,
                    )?;
                    println!(
                        "sustained path={} scenario={} voices={} frames={} p99.9={:.3}% max={:.3}% misses={} throttle={} -> {}",
                        path.as_str(),
                        record.config.scenario.as_str(),
                        voice_count,
                        record.config.frames,
                        record.timing.p999_period_utilization_percent,
                        record.timing.maximum_period_utilization_percent,
                        record.timing.deadline_misses,
                        record.start.throttled,
                        record.end.throttled
                    );
                    let passes = record.callback_pass() && record.no_new_throttling();
                    sustained_tested.push(voice_count);
                    records.push(record);
                    if passes {
                        final_cap = Some(voice_count);
                        break;
                    }
                }
            }
            safety_rows.push(SafetyRow {
                path,
                matrix_cap,
                sustained_tested,
                final_cap,
                boundary: if path == RenderPath::ModelDIdealized {
                    "isolated candidate bank; rapid control is not applicable"
                } else {
                    "production Engine callback simulation"
                },
            });
        }
    } else {
        safety_rows.push(SafetyRow {
            path: RenderPath::ProductionEngine,
            matrix_cap: largest_matrix_cap(&records, RenderPath::ProductionEngine, settings.trials),
            sustained_tested: Vec::new(),
            final_cap: largest_matrix_cap(&records, RenderPath::ProductionEngine, settings.trials),
            boundary: "smoke evidence only",
        });
    }

    mark_determinism(&mut records);
    write_summary(&output, &records)?;
    write_system_snapshots(&output, &records)?;
    write_safety(&output, &safety_rows)?;
    Ok(())
}

fn usage() -> String {
    "usage: native-pi-bench suite OUTPUT [--trial-seconds 30] [--trials 3] [--sustain-seconds 600] [--warmup-audio-seconds 2]\n       native-pi-bench soak OUTPUT PATH SCENARIO VOICES FRAMES\n       native-pi-bench smoke OUTPUT".to_owned()
}

fn parse_soak_request(arguments: Vec<String>) -> Result<SoakRequest, String> {
    let [path, scenario, voices, frames] = arguments.as_slice() else {
        return Err(usage());
    };
    let path = match path.as_str() {
        "production-engine" => RenderPath::ProductionEngine,
        "model-d-idealized" => RenderPath::ModelDIdealized,
        _ => return Err("invalid soak render path".to_owned()),
    };
    let scenario = match scenario.as_str() {
        "held-note" => Scenario::HeldNote,
        "full-chord" => Scenario::FullChord,
        "voice-stealing" => Scenario::VoiceStealing,
        "rapid-control" => Scenario::RapidControl,
        _ => return Err("invalid soak scenario".to_owned()),
    };
    let voices = voices
        .parse::<usize>()
        .map_err(|_| "invalid soak voice count".to_owned())?;
    if !VOICE_COUNTS.contains(&voices) {
        return Err("soak voice count must be one of 1, 2, 4, or 8".to_owned());
    }
    let frames = frames
        .parse::<usize>()
        .map_err(|_| "invalid soak frame count".to_owned())?;
    if !callback_frames().contains(&frames) {
        return Err("soak frame count must be 64, 128, or the live JACK period".to_owned());
    }
    let request = SoakRequest {
        path,
        scenario,
        voices,
        frames,
    };
    if !(CaseConfig {
        path,
        scenario,
        sample_rate: SAMPLE_RATE,
        frames,
        voices,
        engine_macros: EngineMacroConfig::MIDPOINT,
    })
    .is_applicable()
    {
        return Err("the isolated Model-D path has no rapid-control scenario".to_owned());
    }
    Ok(request)
}

fn run_soak(output: &Path, request: SoakRequest) -> Result<(), String> {
    prepare_output(output)?;
    write_platform(output)?;
    let worst_macros = run_engine_prescan(output, 4_096)?;
    write_model_d_prescan(output)?;
    let config = CaseConfig {
        path: request.path,
        scenario: request.scenario,
        sample_rate: SAMPLE_RATE,
        frames: request.frames,
        voices: request.voices,
        engine_macros: worst_macros,
    };
    let settings = RunSettings {
        trial_seconds: 600,
        trials: 1,
        sustained_seconds: 0,
        warmup_audio_seconds: 2,
        paced: true,
        callback_override: None,
        smoke: false,
    };
    let record = run_trial(output, config, 1, true, settings)?;
    println!(
        "sustained path={} scenario={} voices={} frames={} p99.9={:.3}% max={:.3}% misses={} throttle={} -> {}",
        request.path.as_str(),
        request.scenario.as_str(),
        request.voices,
        request.frames,
        record.timing.p999_period_utilization_percent,
        record.timing.maximum_period_utilization_percent,
        record.timing.deadline_misses,
        record.start.throttled,
        record.end.throttled
    );
    let passes = record.callback_pass() && record.no_new_throttling();
    let rows = [SafetyRow {
        path: request.path,
        matrix_cap: None,
        sustained_tested: vec![request.voices],
        final_cap: passes.then_some(request.voices),
        boundary: if request.path == RenderPath::ModelDIdealized {
            "isolated candidate bank soak; matrix evidence is separate"
        } else {
            "production Engine callback-simulation soak; matrix evidence is separate"
        },
    }];
    write_summary(output, std::slice::from_ref(&record))?;
    write_system_snapshots(output, std::slice::from_ref(&record))?;
    write_safety(output, &rows)
}

fn parse_suite_settings(arguments: Vec<String>) -> Result<RunSettings, String> {
    let mut settings = RunSettings {
        trial_seconds: 30,
        trials: 3,
        sustained_seconds: 600,
        warmup_audio_seconds: 2,
        paced: true,
        callback_override: None,
        smoke: false,
    };
    let mut index = 0;
    while index < arguments.len() {
        let name = &arguments[index];
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {name}"))?;
        match name.as_str() {
            "--trial-seconds" => {
                settings.trial_seconds = value.parse().map_err(|_| "invalid trial seconds")?
            }
            "--trials" => settings.trials = value.parse().map_err(|_| "invalid trial count")?,
            "--sustain-seconds" => {
                settings.sustained_seconds =
                    value.parse().map_err(|_| "invalid sustained seconds")?
            }
            "--warmup-audio-seconds" => {
                settings.warmup_audio_seconds =
                    value.parse().map_err(|_| "invalid warmup seconds")?
            }
            _ => return Err(format!("unknown option {name}")),
        }
        index += 2;
    }
    if settings.trial_seconds < 30 || settings.trials < 3 || settings.sustained_seconds < 600 {
        return Err(
            "suite requires at least three 30-second trials and a 600-second sustained trial"
                .to_owned(),
        );
    }
    Ok(settings)
}

fn prepare_output(output: &Path) -> Result<(), String> {
    if output.exists()
        && fs::read_dir(output)
            .map_err(|error| error.to_string())?
            .next()
            .is_some()
    {
        return Err(format!(
            "output directory is not empty: {}",
            output.display()
        ));
    }
    fs::create_dir_all(output.join("raw")).map_err(|error| error.to_string())
}

fn callback_frames() -> Vec<usize> {
    let mut frames = BTreeSet::from(DEFAULT_FRAMES);
    if let Some(actual) = command_output("jack_bufsize", &[])
        .and_then(|value| value.lines().last()?.trim().parse::<usize>().ok())
    {
        frames.insert(actual);
    }
    frames.into_iter().collect()
}

fn run_engine_prescan(output: &Path, callback_count: usize) -> Result<EngineMacroConfig, String> {
    let path = output.join("prescan-engine.tsv");
    let mut writer = BufWriter::new(File::create(path).map_err(|error| error.to_string())?);
    writeln!(
        writer,
        "shape\tcolor\tedge\tcouple\tcallbacks\tmean_ns\tp99_ns\tmaximum_ns\tpeak\tnon_finite"
    )
    .map_err(|error| error.to_string())?;
    let mut worst = EngineMacroConfig::MIDPOINT;
    let mut worst_p99 = 0;
    let mut worst_mean = 0.0;
    for bits in 0_u8..16 {
        let macros = EngineMacroConfig {
            shape: f32::from(bits & 1),
            color: f32::from((bits >> 1) & 1),
            edge: f32::from((bits >> 2) & 1),
            couple: f32::from((bits >> 3) & 1),
        };
        let config = CaseConfig {
            path: RenderPath::ProductionEngine,
            scenario: Scenario::FullChord,
            sample_rate: SAMPLE_RATE,
            frames: 64,
            voices: 8,
            engine_macros: macros,
        };
        let mut case = PreparedCase::new(config).map_err(|error| error.to_string())?;
        for _ in 0..256 {
            case.render_callback().map_err(|error| error.to_string())?;
        }
        let mut durations = initialized_measurement_storage(callback_count);
        let mut peak = 0.0_f32;
        let mut non_finite = 0_u64;
        for duration in &mut durations {
            let start = Instant::now();
            case.render_callback().map_err(|error| error.to_string())?;
            *duration = nanos_u64(start.elapsed());
            verify_output(&case, &mut peak, &mut non_finite, &mut FNV_OFFSET.clone());
        }
        let summary =
            TimingSummary::from_durations(&durations, period_ns(64)).map_err(|e| e.to_string())?;
        writeln!(
            writer,
            "{:.1}\t{:.1}\t{:.1}\t{:.1}\t{}\t{:.3}\t{}\t{}\t{:.9}\t{}",
            macros.shape,
            macros.color,
            macros.edge,
            macros.couple,
            callback_count,
            summary.mean_ns,
            summary.p99_ns,
            summary.maximum_ns,
            peak,
            non_finite
        )
        .map_err(|error| error.to_string())?;
        if summary.p99_ns > worst_p99
            || (summary.p99_ns == worst_p99 && summary.mean_ns > worst_mean)
        {
            worst = macros;
            worst_p99 = summary.p99_ns;
            worst_mean = summary.mean_ns;
        }
    }
    writeln!(
        writer,
        "# selected\t{:.1}\t{:.1}\t{:.1}\t{:.1}\tp99_ns={worst_p99}\tmean_ns={worst_mean:.3}",
        worst.shape, worst.color, worst.edge, worst.couple
    )
    .map_err(|error| error.to_string())?;
    Ok(worst)
}

fn write_model_d_prescan(output: &Path) -> Result<(), String> {
    fs::write(
        output.join("prescan-model-d.tsv"),
        "source\tparameter_surface\tresult\n04_matched_idealized_path.wav\tfixed authored bass patch and idealized diagnostics\tno accepted live parameter extremes; no alternate configuration benchmarked\n",
    )
    .map_err(|error| error.to_string())
}

fn run_trial(
    output: &Path,
    config: CaseConfig,
    trial: usize,
    sustained: bool,
    settings: RunSettings,
) -> Result<TrialRecord, String> {
    let callback_count = settings.callback_override.unwrap_or_else(|| {
        (settings.trial_seconds as usize)
            .saturating_mul(config.sample_rate as usize)
            .div_ceil(config.frames)
    });
    let mut case = PreparedCase::new(config).map_err(|error| error.to_string())?;
    let warmup_callbacks = (settings.warmup_audio_seconds as usize)
        .saturating_mul(config.sample_rate as usize)
        .div_ceil(config.frames);
    for _ in 0..warmup_callbacks {
        case.render_callback().map_err(|error| error.to_string())?;
        black_box(case.output());
    }

    let mut durations = initialized_measurement_storage(callback_count);
    let mut start_lateness = initialized_measurement_storage(callback_count);
    let mut output_peak = 0.0_f32;
    let mut non_finite_count = 0_u64;
    let mut output_hash = FNV_OFFSET;
    let start_snapshot = SystemSnapshot::capture();
    let wall_start = Instant::now();
    for callback in 0..callback_count {
        let expected_start_ns = schedule_ns(callback, config.frames, config.sample_rate);
        let actual_start_ns = nanos_u64(wall_start.elapsed());
        start_lateness[callback] = actual_start_ns.saturating_sub(expected_start_ns);

        let callback_start = Instant::now();
        case.render_callback().map_err(|error| error.to_string())?;
        let elapsed = nanos_u64(callback_start.elapsed());
        durations[callback] = elapsed;
        black_box(case.output());
        black_box(case.right_output());
        verify_output(
            &case,
            &mut output_peak,
            &mut non_finite_count,
            &mut output_hash,
        );

        if settings.paced {
            let next_ns = schedule_ns(callback + 1, config.frames, config.sample_rate);
            let now_ns = nanos_u64(wall_start.elapsed());
            if next_ns > now_ns {
                thread::sleep(Duration::from_nanos(next_ns - now_ns));
            }
        }
    }
    let elapsed_seconds = wall_start.elapsed().as_secs_f64();
    let end_snapshot = SystemSnapshot::capture();
    let timing = TimingSummary::from_durations(&durations, period_ns(config.frames))
        .map_err(|error| error.to_string())?;
    let maximum_start_lateness_ns = start_lateness.iter().copied().max().unwrap_or(0);
    let scheduled_late_count = start_lateness.iter().filter(|value| **value > 0).count();
    let raw_path = raw_path(output, config, trial, sustained);
    write_raw(&raw_path, &durations, &start_lateness)?;
    let expected_active_voice_count = if config.scenario == Scenario::HeldNote {
        1
    } else {
        config.voices
    };
    let per_voice_bound = match config.path {
        RenderPath::ProductionEngine => 1.0,
        RenderPath::ModelDIdealized => 4.0,
    };
    Ok(TrialRecord {
        config,
        trial,
        sustained,
        requested_seconds: settings.trial_seconds,
        elapsed_seconds,
        period_ns: period_ns(config.frames),
        timing,
        scheduled_late_count,
        maximum_start_lateness_ns,
        output_peak,
        non_finite_count,
        output_hash,
        active_voice_count: case.active_voice_count(),
        expected_active_voice_count,
        output_bound: expected_active_voice_count as f32 * per_voice_bound,
        start: start_snapshot,
        end: end_snapshot,
        deterministic: true,
        raw_path,
    })
}

fn initialized_measurement_storage(callback_count: usize) -> Vec<u64> {
    let mut storage = vec![0; callback_count];
    for (index, value) in storage.iter_mut().enumerate() {
        *value = index as u64;
        black_box(*value);
    }
    storage.fill(0);
    storage
}

fn verify_output(case: &PreparedCase, peak: &mut f32, non_finite: &mut u64, hash: &mut u64) {
    for (&left, &right) in case.output().iter().zip(case.right_output()) {
        for sample in [left, right] {
            if sample.is_finite() {
                *peak = peak.max(sample.abs());
            } else {
                *non_finite = non_finite.saturating_add(1);
            }
            for byte in sample.to_bits().to_le_bytes() {
                *hash ^= u64::from(byte);
                *hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
    }
}

fn schedule_ns(callback: usize, frames: usize, sample_rate: u32) -> u64 {
    let numerator = (callback as u128)
        .saturating_mul(frames as u128)
        .saturating_mul(1_000_000_000);
    (numerator / u128::from(sample_rate)).min(u128::from(u64::MAX)) as u64
}

fn period_ns(frames: usize) -> u64 {
    schedule_ns(1, frames, SAMPLE_RATE)
}

fn nanos_u64(duration: Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

fn raw_path(output: &Path, config: CaseConfig, trial: usize, sustained: bool) -> PathBuf {
    let suffix = if sustained { "_sustained" } else { "" };
    output.join("raw").join(format!(
        "{}_{}_v{}_f{}_t{}{}.tsv",
        config.path.as_str(),
        config.scenario.as_str(),
        config.voices,
        config.frames,
        trial,
        suffix
    ))
}

fn write_raw(path: &Path, durations: &[u64], lateness: &[u64]) -> Result<(), String> {
    let mut writer = BufWriter::new(File::create(path).map_err(|error| error.to_string())?);
    writeln!(writer, "callback_index\tduration_ns\tstart_lateness_ns")
        .map_err(|error| error.to_string())?;
    for (index, (&duration, &late)) in durations.iter().zip(lateness).enumerate() {
        writeln!(writer, "{index}\t{duration}\t{late}").map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn mark_determinism(records: &mut [TrialRecord]) {
    for index in 0..records.len() {
        if records[index].sustained {
            continue;
        }
        let reference = records[index].output_hash;
        let config = records[index].config;
        let deterministic = records
            .iter()
            .filter(|candidate| {
                !candidate.sustained
                    && candidate.config.path == config.path
                    && candidate.config.scenario == config.scenario
                    && candidate.config.voices == config.voices
                    && candidate.config.frames == config.frames
            })
            .all(|candidate| candidate.output_hash == reference);
        records[index].deterministic = deterministic;
    }
}

fn voice_matrix_passes(
    records: &[TrialRecord],
    path: RenderPath,
    voices: usize,
    expected_trials: usize,
) -> bool {
    let relevant = records
        .iter()
        .filter(|record| {
            !record.sustained && record.config.path == path && record.config.voices == voices
        })
        .collect::<Vec<_>>();
    let scenario_count = if path == RenderPath::ModelDIdealized {
        3
    } else {
        4
    };
    let frame_count = callback_frames().len();
    relevant.len() == scenario_count * frame_count * expected_trials
        && relevant.iter().all(|record| record.callback_pass())
}

fn largest_matrix_cap(
    records: &[TrialRecord],
    path: RenderPath,
    expected_trials: usize,
) -> Option<usize> {
    VOICE_COUNTS
        .into_iter()
        .rev()
        .find(|voices| voice_matrix_passes(records, path, *voices, expected_trials))
}

fn worst_case(records: &[TrialRecord], path: RenderPath, voices: usize) -> Option<&TrialRecord> {
    records
        .iter()
        .filter(|record| {
            !record.sustained && record.config.path == path && record.config.voices == voices
        })
        .max_by(|left, right| {
            left.timing
                .p999_period_utilization_percent
                .total_cmp(&right.timing.p999_period_utilization_percent)
                .then_with(|| {
                    left.timing
                        .maximum_period_utilization_percent
                        .total_cmp(&right.timing.maximum_period_utilization_percent)
                })
        })
}

fn write_summary(output: &Path, records: &[TrialRecord]) -> Result<(), String> {
    let mut writer = BufWriter::new(
        File::create(output.join("summary.tsv")).map_err(|error| error.to_string())?,
    );
    writeln!(
        writer,
        "path\tsource\tscenario\tvoices\tframes\tsample_rate\ttrial\tsustained\trequested_seconds\telapsed_seconds\tcallbacks\tperiod_ns\tmean_ns\tmedian_ns\tp95_ns\tp99_ns\tp999_ns\tmaximum_ns\tmean_util_percent\tp999_util_percent\tmaximum_util_percent\tdeadline_misses\tscheduled_late_count\tmaximum_start_lateness_ns\toutput_peak\toutput_bound\tnon_finite_count\tactive_voices\texpected_active_voices\thash\tdeterministic\trss_start_kib\trss_end_kib\trss_delta_kib\thwm_end_kib\ttemp_start_millicelsius\ttemp_end_millicelsius\tfreq_start_khz\tfreq_end_khz\tthrottled_start\tthrottled_end\tmigrations_delta\tinvoluntary_context_switches_delta\taffinity\tmemory_stable\tpass\traw_path"
    )
    .map_err(|error| error.to_string())?;
    for record in records {
        writeln!(
            writer,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{}\t{}\t{:.3}\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{:.9}\t{:.3}\t{}\t{}\t{}\t{:016x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            record.config.path.as_str(),
            match record.config.path {
                RenderPath::ProductionEngine => "Engine::render_block",
                RenderPath::ModelDIdealized => "04_matched_idealized_path.wav",
            },
            record.config.scenario.as_str(),
            record.config.voices,
            record.config.frames,
            record.config.sample_rate,
            record.trial,
            record.sustained,
            record.requested_seconds,
            record.elapsed_seconds,
            record.timing.callback_count,
            record.period_ns,
            record.timing.mean_ns,
            record.timing.median_ns,
            record.timing.p95_ns,
            record.timing.p99_ns,
            record.timing.p999_ns,
            record.timing.maximum_ns,
            record.timing.mean_period_utilization_percent,
            record.timing.p999_period_utilization_percent,
            record.timing.maximum_period_utilization_percent,
            record.timing.deadline_misses,
            record.scheduled_late_count,
            record.maximum_start_lateness_ns,
            record.output_peak,
            record.output_bound,
            record.non_finite_count,
            record.active_voice_count,
            record.expected_active_voice_count,
            record.output_hash,
            record.deterministic,
            record.start.rss_kib,
            record.end.rss_kib,
            record.end.rss_kib - record.start.rss_kib,
            record.end.hwm_kib,
            record.start.temperature_millicelsius,
            record.end.temperature_millicelsius,
            record.start.frequency_khz,
            record.end.frequency_khz,
            record.start.throttled,
            record.end.throttled,
            record.end.migrations.saturating_sub(record.start.migrations),
            record
                .end
                .involuntary_context_switches
                .saturating_sub(record.start.involuntary_context_switches),
            record.start.affinity,
            record.memory_stable(),
            record.callback_pass(),
            record.raw_path.display()
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn write_system_snapshots(output: &Path, records: &[TrialRecord]) -> Result<(), String> {
    let mut writer = BufWriter::new(
        File::create(output.join("system-snapshots.tsv")).map_err(|error| error.to_string())?,
    );
    writeln!(
        writer,
        "path\tscenario\tvoices\tframes\ttrial\tsustained\tphase\tepoch_ms\trss_kib\thwm_kib\tvoluntary_context_switches\tinvoluntary_context_switches\tmigrations\ttemperature_millicelsius\tfrequency_khz\tthrottled\taffinity"
    )
    .map_err(|error| error.to_string())?;
    for record in records {
        for (phase, snapshot) in [("start", &record.start), ("end", &record.end)] {
            writeln!(
                writer,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                record.config.path.as_str(),
                record.config.scenario.as_str(),
                record.config.voices,
                record.config.frames,
                record.trial,
                record.sustained,
                phase,
                snapshot.epoch_ms,
                snapshot.rss_kib,
                snapshot.hwm_kib,
                snapshot.voluntary_context_switches,
                snapshot.involuntary_context_switches,
                snapshot.migrations,
                snapshot.temperature_millicelsius,
                snapshot.frequency_khz,
                snapshot.throttled,
                snapshot.affinity
            )
            .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn write_safety(output: &Path, rows: &[SafetyRow]) -> Result<(), String> {
    let mut writer =
        BufWriter::new(File::create(output.join("safety.tsv")).map_err(|error| error.to_string())?);
    writeln!(
        writer,
        "path\tmatrix_provisional_cap\tsustained_counts_tested\tfinal_provisional_cap\tevidence_boundary"
    )
    .map_err(|error| error.to_string())?;
    for row in rows {
        let counts = row
            .sustained_tested
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",");
        writeln!(
            writer,
            "{}\t{}\t{}\t{}\t{}",
            row.path.as_str(),
            optional_count(row.matrix_cap),
            counts,
            optional_count(row.final_cap),
            row.boundary
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn optional_count(value: Option<usize>) -> String {
    value.map_or_else(|| "none".to_owned(), |count| count.to_string())
}

fn write_platform(output: &Path) -> Result<(), String> {
    let platform = detect_platform().map_err(|error| error.to_string())?;
    let mut text = String::new();
    writeln!(text, "label={}", platform.label).unwrap();
    writeln!(text, "architecture={}", platform.architecture).unwrap();
    writeln!(text, "kernel={}", platform.kernel).unwrap();
    writeln!(text, "cpu_model={}", platform.cpu_model).unwrap();
    for (key, command, arguments) in [
        ("uname", "uname", vec!["-a"]),
        ("rustc", "rustc", vec!["-vV"]),
        ("cargo", "cargo", vec!["-V"]),
        ("jack_sample_rate", "jack_samplerate", vec![]),
        ("jack_buffer_frames", "jack_bufsize", vec![]),
        ("throttled", "vcgencmd", vec!["get_throttled"]),
        ("temperature", "vcgencmd", vec!["measure_temp"]),
        ("frequency", "vcgencmd", vec!["measure_clock", "arm"]),
    ] {
        writeln!(
            text,
            "{key}={}",
            command_output(command, &arguments).unwrap_or_else(|| "unavailable".to_owned())
        )
        .unwrap();
    }
    for (key, path) in [
        ("os_release", "/etc/os-release"),
        ("kernel_cmdline", "/proc/cmdline"),
        (
            "cpu_governor",
            "/sys/devices/system/cpu/cpufreq/policy0/scaling_governor",
        ),
        (
            "cpu_min_khz",
            "/sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq",
        ),
        (
            "cpu_max_khz",
            "/sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq",
        ),
        ("isolated_cpus", "/sys/devices/system/cpu/isolated"),
        ("process_status", "/proc/self/status"),
    ] {
        let value = fs::read_to_string(path).unwrap_or_else(|_| "unavailable".to_owned());
        writeln!(text, "{key}={}", value.trim()).unwrap();
    }
    fs::write(output.join("platform.txt"), text).map_err(|error| error.to_string())
}

fn status_value(status: &str, key: &str) -> i64 {
    status
        .lines()
        .find_map(|line| {
            let rest = line.strip_prefix(key)?;
            rest.split_whitespace().next()?.parse().ok()
        })
        .unwrap_or(-1)
}

fn parse_colon_u64(line: &str) -> Option<u64> {
    line.split_once(':')?.1.trim().parse().ok()
}

fn read_i64(path: &str) -> i64 {
    fs::read_to_string(path)
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(-1)
}

fn command_output(command: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(command).args(arguments).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .replace('\n', " | "),
    )
}

#[cfg(test)]
mod tests {
    use super::{RenderPath, Scenario, initialized_measurement_storage, parse_soak_request};

    #[test]
    fn measurement_storage_is_initialized_before_the_timed_loop() {
        let storage = initialized_measurement_storage(32);
        assert_eq!(storage.len(), 32);
        assert!(storage.iter().all(|value| *value == 0));
    }

    #[test]
    fn soak_request_names_an_exact_path_case_and_tested_count() {
        let request = parse_soak_request(vec![
            "production-engine".to_owned(),
            "rapid-control".to_owned(),
            "8".to_owned(),
            "64".to_owned(),
        ])
        .unwrap();
        assert_eq!(request.path, RenderPath::ProductionEngine);
        assert_eq!(request.scenario, Scenario::RapidControl);
        assert_eq!(request.voices, 8);
        assert_eq!(request.frames, 64);
    }
}
