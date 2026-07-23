use assert_no_alloc::assert_no_alloc;
use moj_sint::compact_composite::{
    CompactCandidate, CompactMachine, DriveMethod, ExperimentRole, measure_ablations,
    measure_event, measure_high_rate_residual, measure_variant_retention, render_candidate,
    retained_specs,
};

#[test]
fn retained_specs_limit_layers_and_declare_every_output_stage() {
    let specs = retained_specs();
    assert!((8..=12).contains(&specs.len()));
    assert!(specs.iter().any(|spec| spec.mechanism_count == 1));
    assert!(specs.iter().any(|spec| spec.mechanism_count == 2));
    assert!(specs.iter().any(|spec| spec.mechanism_count == 3));
    for spec in specs {
        assert!(
            (1..=3).contains(&spec.mechanism_count),
            "{:?}",
            spec.candidate
        );
        assert_eq!(
            spec.mechanism_names[..spec.mechanism_count]
                .iter()
                .filter(|name| !name.is_empty())
                .count(),
            spec.mechanism_count
        );
        assert!(
            spec.source_gains[..spec.mechanism_count]
                .iter()
                .all(|gain| gain.is_finite() && *gain > 0.0)
        );
        assert!(spec.drive.is_finite() && spec.drive > 0.0);
        assert!(spec.output_gain.is_finite() && spec.output_gain > 0.0);
        assert_eq!(spec.output_ceiling, 0.999);
        assert!(spec.envelope.is_finite_and_bounded());
        assert!(matches!(
            spec.drive_method,
            DriveMethod::Linear
                | DriveMethod::CubicSaturation
                | DriveMethod::RationalSaturation
                | DriveMethod::HardClip
        ));
    }
}

#[test]
fn prepared_sample_paths_are_deterministic_finite_bounded_and_allocation_free() {
    for candidate in CompactCandidate::ALL {
        let mut first = CompactMachine::new(candidate, 8_000, 38, 0.5, None).unwrap();
        let mut second = CompactMachine::new(candidate, 8_000, 38, 0.5, None).unwrap();
        assert!((1..=3).contains(&first.mechanism_count()));
        for _ in 0..first.duration_frames().min(32_000) {
            let a = assert_no_alloc(|| first.sample());
            let b = second.sample();
            assert_eq!(a, b, "candidate={candidate:?}");
            assert!(a[0].is_finite() && a[1].is_finite());
            assert!(a[0].abs() <= 0.999 && a[1].abs() <= 0.999);
        }
    }
}

#[test]
fn pitch_and_tone_variants_share_one_declared_gain_policy() {
    for candidate in CompactCandidate::ALL {
        let spec = candidate.spec();
        for note in [26, 38, 50] {
            for tone in [
                spec.tone_min,
                0.5 * (spec.tone_min + spec.tone_max),
                spec.tone_max,
            ] {
                let render = render_candidate(candidate, 8_000, note, tone, None).unwrap();
                assert_eq!(render.policy.drive_method, spec.drive_method);
                assert_eq!(render.policy.drive, spec.drive);
                assert_eq!(render.policy.output_gain, spec.output_gain);
                assert_eq!(render.policy.output_ceiling, spec.output_ceiling);
                assert!(render.metrics.finite);
                assert!(render.metrics.peak <= 0.999);
            }
        }
    }
}

#[test]
fn schedules_and_hashes_are_sample_accurate_and_deterministic() {
    for candidate in CompactCandidate::ALL {
        let first =
            render_candidate(candidate, 8_000, candidate.spec().primary_note, 0.5, None).unwrap();
        let second =
            render_candidate(candidate, 8_000, candidate.spec().primary_note, 0.5, None).unwrap();
        assert_eq!(first.samples, second.samples);
        assert_eq!(first.metrics.sample_hash, second.metrics.sample_hash);
        assert_eq!(first.event_frames, second.event_frames);
        assert!(first.event_frames.windows(2).all(|pair| pair[0] < pair[1]));
    }
}

#[test]
fn retained_tone_ranges_do_not_collapse_loudness() {
    for candidate in CompactCandidate::ALL {
        let retention = measure_variant_retention(candidate, 8_000).unwrap();
        assert!(
            retention.tone_rms_db_span <= 4.0,
            "{candidate:?}: {retention:?}"
        );
        assert!(
            retention.tone_loudness_db_span <= 4.0,
            "{candidate:?}: {retention:?}"
        );
        assert!(retention.tone_rows.iter().all(|row| {
            row.metrics.finite
                && row.metrics.peak <= 0.999
                && row.metrics.rms > 0.0
                && row.metrics.perceptual_loudness_db.is_finite()
        }));
    }
}

#[test]
fn bass_pitch_variants_survive_mono_under_one_gain_policy() {
    for candidate in CompactCandidate::ALL {
        let spec = candidate.spec();
        if matches!(
            spec.role,
            ExperimentRole::SustainedLow
                | ExperimentRole::Pedal
                | ExperimentRole::Bass
                | ExperimentRole::BassThump
                | ExperimentRole::SyntheticKick
        ) {
            let retention = measure_variant_retention(candidate, 8_000).unwrap();
            assert!(
                retention.pitch_rows.iter().all(|row| {
                    row.metrics.mono_to_stereo_db >= -1.5
                        && row.metrics.low_side_to_mid <= 0.35
                        && row.fundamental_db.is_finite()
                        && row.mono_fundamental_loss_db <= 6.0
                }),
                "{candidate:?}: {retention:?}"
            );
        }
    }
}

#[test]
fn event_candidates_report_transient_decay_clipping_and_return_to_zero() {
    for candidate in [
        CompactCandidate::ClippedThump,
        CompactCandidate::ResonantThump,
        CompactCandidate::SyntheticKick,
        CompactCandidate::StruckComb,
    ] {
        let render =
            render_candidate(candidate, 48_000, candidate.spec().primary_note, 0.5, None).unwrap();
        let event = measure_event(&render.samples, 48_000, candidate.spec().primary_note);
        assert!(event.window_rms.iter().all(|value| value.is_finite()));
        assert!(event.window_peak.iter().all(|value| value.is_finite()));
        assert!(event.low_band_transient_rms > 0.0);
        assert!(event.onset_to_body_energy.is_finite() && event.onset_to_body_energy > 0.0);
        assert!(event.momentary_loudness_db.is_finite());
        assert!(event.fundamental_start_db.is_finite());
        assert!(event.fundamental_body_db.is_finite());
        assert!(event.decay_60db_ms.is_finite());
        assert!(event.tail_ms > 0.0);
        assert!(event.clipped_proportion >= 0.0);
        assert!(event.maximum_jump.is_finite());
        assert!(event.dc.abs() <= 0.02);
        assert!(event.return_to_zero <= 0.01, "{candidate:?}: {event:?}");
    }
}

#[test]
fn every_retained_mechanism_has_ablation_and_nonlinear_residual_evidence() {
    for candidate in CompactCandidate::ALL {
        let rows = measure_ablations(candidate, 8_000).unwrap();
        assert_eq!(rows.len(), candidate.spec().mechanism_count);
        assert!(
            rows.iter().all(|row| {
                row.difference_db > -24.0
                    && (row.loudness_change_db.abs() > 0.1 || row.early_difference_db > -18.0)
                    && row.muted_hash != row.complete_hash
            }),
            "{candidate:?}: {rows:?}"
        );
        if candidate.spec().drive_method != DriveMethod::Linear {
            let residual = measure_high_rate_residual(
                candidate,
                candidate.spec().primary_note,
                0.5,
                4_000,
                512,
            )
            .unwrap();
            assert!(
                residual.is_finite() && residual <= 3.0,
                "{candidate:?}: {residual}"
            );
        }
    }
}

#[test]
fn retained_full_renders_control_dc_before_the_explicit_drive_stage() {
    for candidate in CompactCandidate::ALL {
        let spec = candidate.spec();
        let render = render_candidate(
            candidate,
            48_000,
            spec.primary_note,
            0.5 * (spec.tone_min + spec.tone_max),
            None,
        )
        .unwrap();
        assert!(
            render.metrics.dc.abs() <= 0.01,
            "{candidate:?}: dc={}",
            render.metrics.dc
        );
    }
}

#[test]
fn musical_context_pitch_variants_transpose_the_score_deterministically() {
    let candidate = CompactCandidate::ContextRelay;
    let mut hashes = Vec::new();
    for note in [26, 38, 50] {
        hashes.push(
            render_candidate(candidate, 8_000, note, 0.5, None)
                .unwrap()
                .metrics
                .sample_hash,
        );
    }
    hashes.sort_unstable();
    hashes.dedup();
    assert_eq!(hashes.len(), 3);
}

#[test]
fn pitch_drop_reaches_the_declared_base_pitch_on_the_exact_sample() {
    for candidate in [
        CompactCandidate::ClippedThump,
        CompactCandidate::ResonantThump,
        CompactCandidate::SyntheticKick,
    ] {
        let spec = candidate.spec();
        let sample_rate = 48_000;
        let mut machine =
            CompactMachine::new(candidate, sample_rate, spec.primary_note, 0.5, None).unwrap();
        let drop_frames =
            (spec.envelope.pitch_drop_ms * sample_rate as f32 / 1_000.0).round() as usize;
        assert!(machine.current_pitch_multiplier() > 1.0);
        for _ in 0..drop_frames {
            machine.sample();
        }
        assert_eq!(machine.current_pitch_multiplier(), 1.0, "{candidate:?}");
    }
}
