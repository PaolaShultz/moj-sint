use moj_sint::compact_composite::{
    AblationEvidence, CompactCandidate, CompactMetrics, CompactRender, EventMetrics,
    ExperimentRole, VariantRetention, fold_to_mono, measure, measure_ablations, measure_event,
    measure_high_rate_residual, measure_variant_retention, midi_frequency, render_candidate,
    retained_specs,
};
use moj_sint::composite_machine::{
    ACTIVE_BODY_END_SECONDS, ACTIVE_BODY_START_SECONDS, CompositeCandidate, render_composite,
    render_hot_composite,
};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

const EXPECTED_SYNCHRONIZED_HASH: u64 = 0xbe3e_a0fd_d66b_4472;
const EXPECTED_DELAYED_HASH: u64 = 0xc919_cb57_920c_520f;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, output] if command == "render" => finish(render_lab(Path::new(output), 48_000)),
        [command, output] if command == "render-test" => {
            finish(render_lab(Path::new(output), 4_000))
        }
        _ => {
            eprintln!("Usage: composite-machine-lab <render|render-test> <output-directory>");
            ExitCode::FAILURE
        }
    }
}

fn finish(result: Result<(), Box<dyn std::error::Error>>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

struct PrimaryRow {
    candidate: CompactCandidate,
    filename: String,
    render: CompactRender,
    active_metrics: CompactMetrics,
}

fn render_lab(output: &Path, sample_rate: u32) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output)?;
    let started = Instant::now();
    let reference = render_hot_composite(CompositeCandidate::DelayedLaunchEstimate, sample_rate)?;
    write_wav(
        &output.join("01_reference_hot-delayed-launch.wav"),
        sample_rate,
        &reference.samples,
    )?;
    let reference_active = active_metrics(
        &reference.samples,
        sample_rate,
        ACTIVE_BODY_START_SECONDS,
        ACTIVE_BODY_END_SECONDS,
    );

    let mut primary = Vec::with_capacity(CompactCandidate::ALL.len());
    for (number, candidate) in CompactCandidate::ALL.into_iter().enumerate() {
        let spec = candidate.spec();
        let tone = 0.5 * (spec.tone_min + spec.tone_max);
        let render = render_candidate(candidate, sample_rate, spec.primary_note, tone, None)?;
        let filename = primary_filename(number + 2, candidate);
        write_wav(&output.join(&filename), sample_rate, &render.samples)?;
        let active = primary_active_metrics(candidate, &render.samples, sample_rate);
        primary.push(PrimaryRow {
            candidate,
            filename,
            render,
            active_metrics: active,
        });
    }

    for candidate in CompactCandidate::ALL {
        if is_bass_bearing(candidate.spec().role) {
            let row = primary
                .iter()
                .find(|row| row.candidate == candidate)
                .expect("every retained candidate has one primary render");
            write_wav(
                &output.join(format!(
                    "12_mono_{}_{}.wav",
                    candidate.spec().slug,
                    note_slug(candidate.spec().primary_note)
                )),
                sample_rate,
                &fold_to_mono(&row.render.samples),
            )?;
        }
    }

    let mut variants = Vec::with_capacity(CompactCandidate::ALL.len());
    let mut ablations = Vec::with_capacity(CompactCandidate::ALL.len());
    for candidate in CompactCandidate::ALL {
        variants.push((
            candidate,
            measure_variant_retention(candidate, sample_rate)?,
        ));
        ablations.push((candidate, measure_ablations(candidate, sample_rate)?));
    }

    write_settings(output)?;
    write_primary_manifest(output, &primary)?;
    write_loudness(output, &primary, reference_active)?;
    write_tone_retention(output, &variants)?;
    write_pitch_retention(output, &variants)?;
    write_event_metrics(output, sample_rate, &primary)?;
    write_ablations(output, &ablations)?;
    write_stereo_mono(output, sample_rate, &primary)?;
    write_residual(output, sample_rate)?;
    write_hashes(
        output,
        reference.raw_hash,
        reference.metrics.sample_hash,
        &primary,
    )?;
    write_rejections(output, reference_active, &primary, &variants, &ablations)?;
    write_reconstruction_regression(output, sample_rate, reference.raw_hash)?;
    write_readme(output, sample_rate, &primary)?;

    let total_wav_seconds = total_wav_seconds(output)?;
    let wall_seconds = started.elapsed().as_secs_f64();
    write_generation_summary(output, total_wav_seconds)?;
    write_cost(output, total_wav_seconds, wall_seconds)?;
    println!(
        "wrote compact composite power lab at {} Hz to {}",
        sample_rate,
        output.display()
    );
    Ok(())
}

fn primary_filename(number: usize, candidate: CompactCandidate) -> String {
    let spec = candidate.spec();
    match spec.role {
        ExperimentRole::SustainedLow => format!(
            "{number:02}_sustained-low_{}_{}.wav",
            spec.slug,
            note_slug(spec.primary_note)
        ),
        ExperimentRole::PlayableMid => format!(
            "{number:02}_playable-mid_{}_{}.wav",
            spec.slug,
            note_slug(spec.primary_note)
        ),
        ExperimentRole::Evolving => format!(
            "{number:02}_evolving_{}_{}.wav",
            spec.slug,
            note_slug(spec.primary_note)
        ),
        ExperimentRole::Pedal => format!(
            "{number:02}_pedal_{}_{}.wav",
            spec.slug,
            note_slug(spec.primary_note)
        ),
        ExperimentRole::Bass => format!(
            "{number:02}_bass_{}_{}.wav",
            spec.slug,
            note_slug(spec.primary_note)
        ),
        ExperimentRole::BassThump => format!(
            "{number:02}_thump_{}_{}.wav",
            spec.slug,
            note_slug(spec.primary_note)
        ),
        ExperimentRole::SyntheticKick => format!(
            "{number:02}_kick_{}_{}.wav",
            spec.slug,
            note_slug(spec.primary_note)
        ),
        ExperimentRole::Struck => format!(
            "{number:02}_struck_{}_{}.wav",
            spec.slug,
            note_slug(spec.primary_note)
        ),
        ExperimentRole::MusicalContext => {
            format!("{number:02}_context_{}.wav", spec.slug)
        }
    }
}

fn primary_active_metrics(
    candidate: CompactCandidate,
    samples: &[f32],
    sample_rate: u32,
) -> CompactMetrics {
    let duration = samples.len() as f64 / (2.0 * f64::from(sample_rate));
    match candidate.spec().role {
        ExperimentRole::BassThump | ExperimentRole::SyntheticKick | ExperimentRole::Struck => {
            active_metrics(samples, sample_rate, 0.0, duration.min(0.4))
        }
        _ => active_metrics(samples, sample_rate, 0.25, (duration - 0.25).max(0.26)),
    }
}

fn active_metrics(
    samples: &[f32],
    sample_rate: u32,
    start_seconds: f64,
    end_seconds: f64,
) -> CompactMetrics {
    let frames = samples.len() / 2;
    let start = ((start_seconds * f64::from(sample_rate)).round() as usize).min(frames);
    let end = ((end_seconds * f64::from(sample_rate)).round() as usize).min(frames);
    measure(&samples[2 * start..2 * end.max(start + 1)], sample_rate)
}

fn write_settings(output: &Path) -> std::io::Result<()> {
    let mut file = writer(output, "settings.tsv")?;
    writeln!(
        file,
        "candidate\trole\tmechanisms\tmechanism_count\tsource_gains\tstereo_widths\tattack_ms\tdecay_ms\tsustain\trelease_ms\tgate_ms\tpitch_drop_semitones\tpitch_drop_ms\tdrive_method\tdrive\tpre_and_post_drive_dc_block_hz\toutput_gain\toutput_ceiling\ttone_min\ttone_max\tprimary_midi\tprimary_hz\tduration_seconds\tgain_policy"
    )?;
    for spec in retained_specs() {
        writeln!(
            file,
            "{}\t{:?}\t{}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{:.6}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{}\t{:.6}\t{:.3}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{:.6}\t{:.3}\tfixed_across_pitch_and_tone_no_per_file_normalization",
            spec.slug,
            spec.role,
            spec.mechanism_names[..spec.mechanism_count].join("+"),
            spec.mechanism_count,
            join_f32(&spec.source_gains[..spec.mechanism_count]),
            join_f32(&spec.stereo_widths[..spec.mechanism_count]),
            spec.envelope.attack_ms,
            spec.envelope.decay_ms,
            spec.envelope.sustain,
            spec.envelope.release_ms,
            spec.envelope.gate_ms,
            spec.envelope.pitch_drop_semitones,
            spec.envelope.pitch_drop_ms,
            spec.drive_method.slug(),
            spec.drive,
            spec.dc_block_hz,
            spec.output_gain,
            spec.output_ceiling,
            spec.tone_min,
            spec.tone_max,
            spec.primary_note,
            midi_frequency(spec.primary_note),
            spec.duration_seconds,
        )?;
    }
    Ok(())
}

fn write_primary_manifest(output: &Path, primary: &[PrimaryRow]) -> std::io::Result<()> {
    let mut file = writer(output, "primary-manifest.tsv")?;
    writeln!(
        file,
        "file\tcandidate\trole\tmechanisms\tpeak\tactive_rms\tactive_perceptual_db\tactive_low_rms\tcrest\tdc\tclipped_proportion\tmono_db\thash\tfinite"
    )?;
    for row in primary {
        let spec = row.candidate.spec();
        writeln!(
            file,
            "{}\t{}\t{:?}\t{}\t{:.9}\t{:.9}\t{:.6}\t{:.9}\t{:.6}\t{:.12}\t{:.9}\t{:.6}\t{:016x}\t{}",
            row.filename,
            spec.slug,
            spec.role,
            spec.mechanism_count,
            row.render.metrics.peak,
            row.active_metrics.rms,
            row.active_metrics.perceptual_loudness_db,
            row.active_metrics.low_rms,
            row.render.metrics.crest_factor,
            row.render.metrics.dc,
            row.render.metrics.clipped_proportion,
            row.render.metrics.mono_to_stereo_db,
            row.render.metrics.sample_hash,
            row.render.metrics.finite,
        )?;
    }
    Ok(())
}

fn write_loudness(
    output: &Path,
    primary: &[PrimaryRow],
    reference: CompactMetrics,
) -> std::io::Result<()> {
    let mut file = writer(output, "loudness.tsv")?;
    writeln!(
        file,
        "candidate\twindow\tactive_rms\tactive_rms_vs_reference_db\tshort_term_perceptual_db\tperceptual_vs_reference_db\tlow_rms\tcrest\tpeak\tclipped_proportion"
    )?;
    writeln!(
        file,
        "hot-delayed-reference\t4.25-8.00s\t{:.9}\t0.000\t{:.6}\t0.000\t{:.9}\t{:.6}\t{:.9}\t{:.9}",
        reference.rms,
        reference.perceptual_loudness_db,
        reference.low_rms,
        reference.crest_factor,
        reference.peak,
        reference.clipped_proportion,
    )?;
    for row in primary {
        let event = matches!(
            row.candidate.spec().role,
            ExperimentRole::BassThump | ExperimentRole::SyntheticKick | ExperimentRole::Struck
        );
        writeln!(
            file,
            "{}\t{}\t{:.9}\t{:.3}\t{:.6}\t{:.3}\t{:.9}\t{:.6}\t{:.9}\t{:.9}",
            row.candidate.spec().slug,
            if event {
                "0.00-0.40s event"
            } else {
                "active body"
            },
            row.active_metrics.rms,
            amplitude_db(row.active_metrics.rms / reference.rms.max(1.0e-24)),
            row.active_metrics.perceptual_loudness_db,
            row.active_metrics.perceptual_loudness_db - reference.perceptual_loudness_db,
            row.active_metrics.low_rms,
            row.active_metrics.crest_factor,
            row.active_metrics.peak,
            row.active_metrics.clipped_proportion,
        )?;
    }
    Ok(())
}

fn write_tone_retention(
    output: &Path,
    variants: &[(CompactCandidate, VariantRetention)],
) -> std::io::Result<()> {
    let mut file = writer(output, "tone-retention.tsv")?;
    writeln!(
        file,
        "candidate\tposition\ttone\tmidi\trms\tperceptual_db\tpeak\tlow_rms\tfundamental_db\tmono_fundamental_loss_db\trms_span_db\tperceptual_span_db\tstatus"
    )?;
    for (candidate, retention) in variants {
        for (position, row) in ["minimum", "median", "maximum"]
            .into_iter()
            .zip(&retention.tone_rows)
        {
            writeln!(
                file,
                "{}\t{}\t{:.6}\t{}\t{:.9}\t{:.6}\t{:.9}\t{:.9}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{}",
                candidate.spec().slug,
                position,
                row.tone,
                row.note,
                row.metrics.rms,
                row.metrics.perceptual_loudness_db,
                row.metrics.peak,
                row.metrics.low_rms,
                row.fundamental_db,
                row.mono_fundamental_loss_db,
                retention.tone_rms_db_span,
                retention.tone_loudness_db_span,
                if retention.tone_rms_db_span <= 4.0 && retention.tone_loudness_db_span <= 4.0 {
                    "pass"
                } else {
                    "reject_loudness_collapse"
                },
            )?;
        }
    }
    Ok(())
}

fn write_pitch_retention(
    output: &Path,
    variants: &[(CompactCandidate, VariantRetention)],
) -> std::io::Result<()> {
    let mut file = writer(output, "pitch-retention.tsv")?;
    writeln!(
        file,
        "candidate\tmidi\tfrequency_hz\trms\tperceptual_db\tpeak\tlow_rms\tfundamental_db\tmono_fundamental_loss_db\tmono_db\tlow_side_to_mid\tgain_policy"
    )?;
    for (candidate, retention) in variants {
        for row in &retention.pitch_rows {
            writeln!(
                file,
                "{}\t{}\t{:.6}\t{:.9}\t{:.6}\t{:.9}\t{:.9}\t{:.3}\t{:.3}\t{:.3}\t{:.9}\tfixed",
                candidate.spec().slug,
                row.note,
                midi_frequency(row.note),
                row.metrics.rms,
                row.metrics.perceptual_loudness_db,
                row.metrics.peak,
                row.metrics.low_rms,
                row.fundamental_db,
                row.mono_fundamental_loss_db,
                row.metrics.mono_to_stereo_db,
                row.metrics.low_side_to_mid,
            )?;
        }
    }
    Ok(())
}

fn write_event_metrics(
    output: &Path,
    sample_rate: u32,
    primary: &[PrimaryRow],
) -> std::io::Result<()> {
    let mut file = writer(output, "event-metrics.tsv")?;
    writeln!(
        file,
        "candidate\trms_10ms\tpeak_10ms\trms_25ms\tpeak_25ms\trms_50ms\tpeak_50ms\trms_100ms\tpeak_100ms\trms_250ms\tpeak_250ms\tmomentary_perceptual_db\tonset_to_body_energy\tlow_band_transient_rms\tfundamental_start_db\tfundamental_body_db\tdeclared_pitch_start_hz\tdeclared_pitch_end_hz\tpitch_drop_ms\tdecay_60db_ms\ttail_ms\tclipped_proportion\tmaximum_jump\tdc\treturn_to_zero\tmono_db"
    )?;
    for row in primary {
        if !matches!(
            row.candidate.spec().role,
            ExperimentRole::BassThump | ExperimentRole::SyntheticKick | ExperimentRole::Struck
        ) {
            continue;
        }
        let spec = row.candidate.spec();
        let event = measure_event(&row.render.samples, sample_rate, spec.primary_note);
        write_event_row(
            &mut file,
            row.candidate,
            event,
            row.render.metrics.mono_to_stereo_db,
        )?;
    }
    Ok(())
}

fn write_event_row(
    file: &mut impl Write,
    candidate: CompactCandidate,
    event: EventMetrics,
    mono_db: f64,
) -> std::io::Result<()> {
    let spec = candidate.spec();
    let end_hz = midi_frequency(spec.primary_note);
    let start_hz = end_hz * 2.0_f32.powf(spec.envelope.pitch_drop_semitones / 12.0);
    writeln!(
        file,
        "{}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.6}\t{:.6}\t{:.9}\t{:.3}\t{:.3}\t{:.6}\t{:.6}\t{:.3}\t{:.3}\t{:.3}\t{:.9}\t{:.9}\t{:.12}\t{:.9}\t{:.3}",
        spec.slug,
        event.window_rms[0],
        event.window_peak[0],
        event.window_rms[1],
        event.window_peak[1],
        event.window_rms[2],
        event.window_peak[2],
        event.window_rms[3],
        event.window_peak[3],
        event.window_rms[4],
        event.window_peak[4],
        event.momentary_loudness_db,
        event.onset_to_body_energy,
        event.low_band_transient_rms,
        event.fundamental_start_db,
        event.fundamental_body_db,
        start_hz,
        end_hz,
        spec.envelope.pitch_drop_ms,
        event.decay_60db_ms,
        event.tail_ms,
        event.clipped_proportion,
        event.maximum_jump,
        event.dc,
        event.return_to_zero,
        mono_db,
    )
}

fn write_ablations(
    output: &Path,
    ablations: &[(CompactCandidate, Vec<AblationEvidence>)],
) -> std::io::Result<()> {
    let mut file = writer(output, "ablation.tsv")?;
    writeln!(
        file,
        "candidate\tmechanism_index\tmechanism\tfull_difference_db\tearly_difference_db\tloudness_change_db\tcomplete_hash\tmuted_hash\tstatus"
    )?;
    for (candidate, rows) in ablations {
        for row in rows {
            let active = row.difference_db > -24.0
                && (row.loudness_change_db.abs() > 0.1 || row.early_difference_db > -18.0);
            writeln!(
                file,
                "{}\t{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{:016x}\t{:016x}\t{}",
                candidate.spec().slug,
                row.mechanism_index,
                row.mechanism_name,
                row.difference_db,
                row.early_difference_db,
                row.loudness_change_db,
                row.complete_hash,
                row.muted_hash,
                if active { "active" } else { "reject_inert" },
            )?;
        }
    }
    Ok(())
}

fn write_stereo_mono(
    output: &Path,
    sample_rate: u32,
    primary: &[PrimaryRow],
) -> std::io::Result<()> {
    let mut file = writer(output, "stereo-mono.tsv")?;
    writeln!(
        file,
        "candidate\tcorrelation\tside_to_mid\tlow_side_to_mid\tstereo_rms\tmono_rms\tmono_db\tstereo_fundamental_db\tmono_fundamental_db\tfundamental_loss_db\tcancellation_status"
    )?;
    for row in primary {
        let mono_samples = fold_to_mono(&row.render.samples);
        let mono = measure(&mono_samples, sample_rate);
        let frequency = midi_frequency(row.candidate.spec().primary_note);
        let stereo_fundamental =
            moj_sint::compact_composite::projection_db(&row.render.samples, sample_rate, frequency);
        let mono_fundamental =
            moj_sint::compact_composite::projection_db(&mono_samples, sample_rate, frequency);
        let loss = (stereo_fundamental - mono_fundamental).max(0.0);
        writeln!(
            file,
            "{}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{}",
            row.candidate.spec().slug,
            row.render.metrics.correlation,
            row.render.metrics.side_to_mid,
            row.render.metrics.low_side_to_mid,
            row.render.metrics.rms,
            mono.rms,
            row.render.metrics.mono_to_stereo_db,
            stereo_fundamental,
            mono_fundamental,
            loss,
            if row.render.metrics.mono_to_stereo_db >= -1.5 && loss <= 6.0 {
                "pass"
            } else {
                "reject_cancellation"
            },
        )?;
    }
    Ok(())
}

fn write_residual(output: &Path, sample_rate: u32) -> std::io::Result<()> {
    let mut file = writer(output, "residual.tsv")?;
    writeln!(
        file,
        "candidate\tdrive_method\tresidual_db\treference\tframes\tinterpretation"
    )?;
    let frames = if sample_rate == 48_000 { 2_048 } else { 512 };
    for candidate in CompactCandidate::ALL {
        let spec = candidate.spec();
        let residual = measure_high_rate_residual(
            candidate,
            spec.primary_note,
            0.5 * (spec.tone_min + spec.tone_max),
            sample_rate,
            frames,
        )
        .map_err(io_other)?;
        writeln!(
            file,
            "{}\t{}\t{:.3}\t4x_box_decimated_gain_fitted\t{}\tconservative amplitude phase nonlinear and state-rate residual; not audibility",
            spec.slug,
            spec.drive_method.slug(),
            residual,
            frames,
        )?;
    }
    Ok(())
}

fn write_hashes(
    output: &Path,
    reference_raw_hash: u64,
    reference_hot_hash: u64,
    primary: &[PrimaryRow],
) -> std::io::Result<()> {
    let mut file = writer(output, "hashes.tsv")?;
    writeln!(file, "file\tcandidate\thash")?;
    writeln!(
        file,
        "01_reference_hot-delayed-launch.wav\thot-delayed-reference\t{:016x}",
        reference_hot_hash
    )?;
    writeln!(
        file,
        "raw-reference-not-written\tdelayed-launch-raw\t{:016x}",
        reference_raw_hash
    )?;
    for row in primary {
        writeln!(
            file,
            "{}\t{}\t{:016x}",
            row.filename,
            row.candidate.spec().slug,
            row.render.metrics.sample_hash
        )?;
    }
    Ok(())
}

fn write_rejections(
    output: &Path,
    reference: CompactMetrics,
    primary: &[PrimaryRow],
    variants: &[(CompactCandidate, VariantRetention)],
    ablations: &[(CompactCandidate, Vec<AblationEvidence>)],
) -> std::io::Result<()> {
    let mut file = writer(output, "rejections.tsv")?;
    writeln!(file, "candidate\trule\tstatus\tvalue\tlimit")?;
    for row in primary {
        rejection(
            &mut file,
            row.candidate,
            "finite",
            row.render.metrics.finite,
            f64::from(row.render.metrics.finite),
            1.0,
        )?;
        rejection(
            &mut file,
            row.candidate,
            "digital_ceiling",
            row.render.metrics.peak <= 0.999,
            row.render.metrics.peak,
            0.999,
        )?;
        if !matches!(
            row.candidate.spec().role,
            ExperimentRole::BassThump | ExperimentRole::SyntheticKick | ExperimentRole::Struck
        ) {
            rejection(
                &mut file,
                row.candidate,
                "sustained_loudness_floor_db",
                row.active_metrics.perceptual_loudness_db >= reference.perceptual_loudness_db - 1.5,
                row.active_metrics.perceptual_loudness_db - reference.perceptual_loudness_db,
                -1.5,
            )?;
        }
        if is_bass_bearing(row.candidate.spec().role) {
            rejection(
                &mut file,
                row.candidate,
                "mono_fold_db",
                row.render.metrics.mono_to_stereo_db >= -1.5,
                row.render.metrics.mono_to_stereo_db,
                -1.5,
            )?;
        }
    }
    for (candidate, retention) in variants {
        rejection(
            &mut file,
            *candidate,
            "tone_rms_span_db",
            retention.tone_rms_db_span <= 4.0,
            retention.tone_rms_db_span,
            4.0,
        )?;
        rejection(
            &mut file,
            *candidate,
            "tone_perceptual_span_db",
            retention.tone_loudness_db_span <= 4.0,
            retention.tone_loudness_db_span,
            4.0,
        )?;
    }
    for (candidate, rows) in ablations {
        for row in rows {
            rejection(
                &mut file,
                *candidate,
                "ablation_difference_db",
                row.difference_db > -24.0
                    && (row.loudness_change_db.abs() > 0.1 || row.early_difference_db > -18.0),
                row.difference_db,
                -24.0,
            )?;
        }
    }
    Ok(())
}

fn rejection(
    file: &mut impl Write,
    candidate: CompactCandidate,
    rule: &str,
    passed: bool,
    value: f64,
    limit: f64,
) -> std::io::Result<()> {
    writeln!(
        file,
        "{}\t{}\t{}\t{:.9}\t{:.9}",
        candidate.spec().slug,
        rule,
        if passed { "pass" } else { "reject" },
        value,
        limit
    )
}

fn write_reconstruction_regression(
    output: &Path,
    sample_rate: u32,
    delayed_raw_hash: u64,
) -> std::io::Result<()> {
    let mut file = writer(output, "reconstruction-regression.tsv")?;
    writeln!(
        file,
        "candidate\tsample_rate\traw_hash\texpected_48k_hash\tstatus"
    )?;
    if sample_rate == 48_000 {
        let synchronized = render_composite(
            CompositeCandidate::SynchronizedReference,
            sample_rate,
            false,
        )
        .map_err(io_other)?;
        for (candidate, actual, expected) in [
            (
                "synchronized-reference",
                synchronized.metrics.sample_hash,
                EXPECTED_SYNCHRONIZED_HASH,
            ),
            (
                "delayed-launch-estimate",
                delayed_raw_hash,
                EXPECTED_DELAYED_HASH,
            ),
        ] {
            writeln!(
                file,
                "{}\t{}\t{:016x}\t{:016x}\t{}",
                candidate,
                sample_rate,
                actual,
                expected,
                if actual == expected {
                    "sample_identical"
                } else {
                    "reject_hash_mismatch"
                }
            )?;
        }
    } else {
        writeln!(
            file,
            "delayed-launch-estimate\t{}\t{:016x}\t{:016x}\tquick_mode_determinism_only",
            sample_rate, delayed_raw_hash, EXPECTED_DELAYED_HASH
        )?;
    }
    Ok(())
}

fn write_readme(output: &Path, sample_rate: u32, primary: &[PrimaryRow]) -> std::io::Result<()> {
    let mut file = writer(output, "README.md")?;
    writeln!(file, "# Moj Sint compact composite power lab\n")?;
    writeln!(
        file,
        "**This batch is intentionally hot; start with playback volume low.** Digital sample level is not acoustic SPL and does not establish safe acoustic playback.\n"
    )?;
    writeln!(
        file,
        "This is a human listening gate, not a claim of musical success. The ten primary experiments use one mechanism, two mechanisms, or three mechanisms; no primary topology exceeds three simultaneous source mechanisms. Every source gain, envelope, drive, saturation or clipping method, output gain, and ceiling is in `settings.tsv`. There is no automatic per-file peak normalization.\n"
    )?;
    writeln!(file, "Listen in this order:\n")?;
    writeln!(
        file,
        "1. `01_reference_hot-delayed-launch.wav` — current hot perceived-power orientation only.\n"
    )?;
    for (index, row) in primary.iter().enumerate() {
        let spec = row.candidate.spec();
        writeln!(
            file,
            "{}. `{}` — {:?}; {} {}.",
            index + 2,
            row.filename,
            spec.role,
            spec.mechanism_count,
            if spec.mechanism_count == 1 {
                "mechanism"
            } else {
                "mechanisms"
            }
        )?;
    }
    writeln!(
        file,
        "\n## Exact primary settings and measurements\n\n| File | Topology | Pitch / role | Envelope ms and level | Source gains | Widths | Drive / DC / output | Peak | Active or event RMS | Perceptual proxy dB | Clip % | Mono dB | Hash |\n| --- | --- | --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- |"
    )?;
    for row in primary {
        let spec = row.candidate.spec();
        writeln!(
            file,
            "| `{}` | {}: {} | MIDI {} ({:.3} Hz), {:?} | {:.2}/{:.2}/{:.3}/{:.2}, gate {:.2}, drop {:.1} st/{:.1} ms | {} | {} | {} {:.3}, DC {:.1} Hz pre/post, out {:.3}, ceiling {:.3} | {:.6} | {:.6} | {:.3} | {:.3} | {:.3} | `{:016x}` |",
            row.filename,
            spec.mechanism_count,
            spec.mechanism_names[..spec.mechanism_count].join(" + "),
            spec.primary_note,
            midi_frequency(spec.primary_note),
            spec.role,
            spec.envelope.attack_ms,
            spec.envelope.decay_ms,
            spec.envelope.sustain,
            spec.envelope.release_ms,
            spec.envelope.gate_ms,
            spec.envelope.pitch_drop_semitones,
            spec.envelope.pitch_drop_ms,
            join_f32(&spec.source_gains[..spec.mechanism_count]),
            join_f32(&spec.stereo_widths[..spec.mechanism_count]),
            spec.drive_method.slug(),
            spec.drive,
            spec.dc_block_hz,
            spec.output_gain,
            spec.output_ceiling,
            row.render.metrics.peak,
            row.active_metrics.rms,
            row.active_metrics.perceptual_loudness_db,
            100.0 * row.render.metrics.clipped_proportion,
            row.render.metrics.mono_to_stereo_db,
            row.render.metrics.sample_hash,
        )?;
    }
    writeln!(
        file,
        "\n## mono diagnostics last\n\nAfter learning the stereo identities, compare the `12_mono_*` files for every bass-bearing experiment. Stereo width is not accepted if perceived power or the fundamental disappears in mono.\n"
    )?;
    writeln!(
        file,
        "Reports contain exact settings, common-window loudness, minimum/median/maximum tone retention, D1/D2/D3 pitch behavior under one fixed gain policy, event windows, clipping proportion, ablations, stereo/mono behavior, conservative high-rate residuals, and deterministic hashes. The perceptual column is a deterministic high-pass/high-frequency-weighted proxy, not calibrated LUFS. Human listening decides power, distinction, and specialness.\n"
    )?;
    writeln!(
        file,
        "This batch is {} Hz. Workstation generation timing is not Raspberry Pi, callback, latency, polyphony, or low-memory evidence.",
        sample_rate
    )
}

fn write_generation_summary(output: &Path, total_wav_seconds: f64) -> std::io::Result<()> {
    let mut file = writer(output, "generation-summary.tsv")?;
    writeln!(file, "metric\tvalue\tunit")?;
    writeln!(file, "total_wav_audio\t{total_wav_seconds:.6}\tseconds")?;
    writeln!(
        file,
        "primary_experiments\t{}\tcount",
        CompactCandidate::ALL.len()
    )?;
    writeln!(file, "maximum_mechanisms\t3\tcount")?;
    writeln!(file, "output_ceiling\t0.999\tfloating_sample")
}

fn write_cost(output: &Path, total_wav_seconds: f64, wall_seconds: f64) -> std::io::Result<()> {
    let mut file = writer(output, "workstation-cost.txt")?;
    writeln!(file, "scope=complete_release_generation")?;
    writeln!(file, "total_wav_audio_seconds={total_wav_seconds:.6}")?;
    writeln!(file, "wall_clock_seconds={wall_seconds:.6}")?;
    writeln!(
        file,
        "audio_duration_to_generation_ratio={:.6}",
        total_wav_seconds / wall_seconds.max(1.0e-12)
    )?;
    writeln!(
        file,
        "equivalent_two_second_wav_seconds={:.6}",
        2.0 * wall_seconds / total_wav_seconds.max(1.0e-12)
    )?;
    writeln!(
        file,
        "limitation=x86_64 workstation offline generation only; not Raspberry Pi callback latency polyphony or memory evidence"
    )
}

fn total_wav_seconds(output: &Path) -> Result<f64, Box<dyn std::error::Error>> {
    let mut total = 0.0;
    for entry in fs::read_dir(output)? {
        let path: PathBuf = entry?.path();
        if path.extension().is_some_and(|extension| extension == "wav") {
            let reader = hound::WavReader::open(path)?;
            total += reader.duration() as f64 / f64::from(reader.spec().sample_rate);
        }
    }
    Ok(total)
}

fn write_wav(path: &Path, sample_rate: u32, samples: &[f32]) -> Result<(), hound::Error> {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;
    for sample in samples {
        writer.write_sample(*sample)?;
    }
    writer.finalize()
}

fn note_slug(note: u8) -> &'static str {
    match note {
        26 => "d1",
        38 => "d2",
        50 => "d3",
        _ => "note",
    }
}

fn is_bass_bearing(role: ExperimentRole) -> bool {
    matches!(
        role,
        ExperimentRole::SustainedLow
            | ExperimentRole::Pedal
            | ExperimentRole::Bass
            | ExperimentRole::BassThump
            | ExperimentRole::SyntheticKick
    )
}

fn join_f32(values: &[f32]) -> String {
    values
        .iter()
        .map(|value| format!("{value:.6}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn amplitude_db(amplitude: f64) -> f64 {
    20.0 * amplitude.max(1.0e-12).log10()
}

fn writer(output: &Path, name: &str) -> std::io::Result<BufWriter<File>> {
    Ok(BufWriter::new(File::create(output.join(name))?))
}

fn io_other(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(error.to_string())
}
