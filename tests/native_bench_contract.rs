use assert_no_alloc::assert_no_alloc;
use moj_sint::native_bench::{
    CaseConfig, EngineMacroConfig, PreparedCase, RenderPath, Scenario, TimingSummary,
    detect_platform,
};

fn config(path: RenderPath, scenario: Scenario, voices: usize, frames: usize) -> CaseConfig {
    CaseConfig {
        path,
        scenario,
        sample_rate: 48_000,
        frames,
        voices,
        engine_macros: EngineMacroConfig::MIDPOINT,
    }
}

#[test]
fn production_engine_callback_is_preallocated_finite_and_deterministic() {
    let config = config(RenderPath::ProductionEngine, Scenario::RapidControl, 4, 64);
    let mut first = PreparedCase::new(config).unwrap();
    let mut second = PreparedCase::new(config).unwrap();

    assert!(first.maximum_event_offset() < config.frames);
    for _ in 0..256 {
        assert_no_alloc(|| first.render_callback().unwrap());
        assert_no_alloc(|| second.render_callback().unwrap());
        assert_eq!(first.output(), second.output());
        assert!(first.output().iter().all(|sample| sample.is_finite()));
    }
    assert_eq!(first.active_voice_count(), 4);
}

#[test]
fn model_d_case_uses_the_exact_idealized_audition_path_without_allocation() {
    let config = config(RenderPath::ModelDIdealized, Scenario::FullChord, 2, 128);
    let mut first = PreparedCase::new(config).unwrap();
    let mut second = PreparedCase::new(config).unwrap();

    assert_eq!(first.render_path_source(), "04_matched_idealized_path.wav");
    for _ in 0..256 {
        assert_no_alloc(|| first.render_callback().unwrap());
        assert_no_alloc(|| second.render_callback().unwrap());
        assert_eq!(first.output(), second.output());
        assert!(first.output().iter().all(|sample| sample.is_finite()));
    }
    assert_eq!(first.active_voice_count(), 2);
}

#[test]
fn repeated_events_keep_fixed_capacity_active_while_stealing() {
    for path in [RenderPath::ProductionEngine, RenderPath::ModelDIdealized] {
        let mut case = PreparedCase::new(config(path, Scenario::VoiceStealing, 4, 64)).unwrap();
        for _ in 0..1_024 {
            assert_no_alloc(|| case.render_callback().unwrap());
            assert!(case.output().iter().all(|sample| sample.is_finite()));
        }
        assert_eq!(case.active_voice_count(), 4);
    }
}

#[test]
fn every_scenario_keeps_events_inside_caller_owned_buffers() {
    for path in [RenderPath::ProductionEngine, RenderPath::ModelDIdealized] {
        for scenario in Scenario::ALL {
            for frames in [64, 128] {
                let config = config(path, scenario, 8, frames);
                if !config.is_applicable() {
                    continue;
                }
                let case = PreparedCase::new(config).unwrap();
                assert!(case.maximum_event_offset() < frames);
                assert!(case.maximum_events_per_block() <= 16);
            }
        }
    }
}

#[test]
fn percentile_summary_uses_nearest_rank_and_reports_period_utilization() {
    let summary = TimingSummary::from_durations(&[10, 20, 30, 40, 50], 100).unwrap();
    assert_eq!(summary.median_ns, 30);
    assert_eq!(summary.p95_ns, 50);
    assert_eq!(summary.p99_ns, 50);
    assert_eq!(summary.p999_ns, 50);
    assert_eq!(summary.maximum_ns, 50);
    assert!((summary.maximum_period_utilization_percent - 50.0).abs() < f64::EPSILON);
}

#[test]
fn platform_detection_reports_the_real_compilation_architecture() {
    let platform = detect_platform().unwrap();
    assert_eq!(platform.architecture, std::env::consts::ARCH);
    assert_ne!(platform.label, "x86_64 workstation");
    assert!(!platform.label.is_empty());
}
