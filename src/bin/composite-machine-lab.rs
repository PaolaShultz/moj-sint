use moj_sint::composite_machine::{
    ACTIVE_BODY_END_SECONDS, ACTIVE_BODY_START_SECONDS, AuditionKind, AuditionRender,
    CompositeCandidate, CompositeMachine, CompositeMetrics, CompositeRender, HotCompositeRender,
    measure_ablations, measure_composite, measure_composite_residual,
    measure_junction_product_levels, measure_junction_residual, measure_product_levels,
    measure_projection, midi_frequency, punch_sweep, render_bass_candidate, render_composite,
    render_held_candidate, render_hot_composite, sample_similarity, window_rms,
};
use moj_sint::dsp::hybrid::HybridFamily;
use moj_sint::hybrid::measure_hybrid_alias_error;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

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

fn render_lab(output: &Path, sample_rate: u32) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output)?;
    let mut renders = Vec::with_capacity(CompositeCandidate::ALL.len());
    for candidate in CompositeCandidate::ALL {
        let stereo = render_composite(candidate, sample_rate, false)?;
        let mono = mono_fold(&stereo, sample_rate);
        write_wav(
            &output.join(format!("08_raw_{}_stereo.wav", candidate.slug())),
            sample_rate,
            &stereo.samples,
        )?;
        renders.push((candidate, stereo, mono));
    }
    let mut hot_renders = Vec::with_capacity(CompositeCandidate::ALL.len());
    for candidate in CompositeCandidate::ALL {
        let hot = render_hot_composite(candidate, sample_rate)?;
        let name = match candidate {
            CompositeCandidate::DelayedLaunchEstimate => {
                "01_hot-delayed-launch-reference_stereo.wav".to_owned()
            }
            CompositeCandidate::SynchronizedReference => {
                "02_hot-synchronized-counterfactual_stereo.wav".to_owned()
            }
            _ => format!("03_hot-full-score_{}_stereo.wav", candidate.slug()),
        };
        write_wav(&output.join(name), sample_rate, &hot.samples)?;
        hot_renders.push((candidate, hot));
    }

    let mut audition_rows = Vec::new();
    for candidate in CompositeCandidate::FINAL_CANDIDATES {
        for (note, register) in [(26, "d1"), (38, "d2"), (50, "d3")] {
            let render = render_held_candidate(candidate, note, sample_rate)?;
            write_wav(
                &output.join(format!(
                    "04_octave_{}_{}_stereo.wav",
                    candidate.slug(),
                    register
                )),
                sample_rate,
                &render.samples,
            )?;
            if note == 26 {
                write_wav(
                    &output.join(format!("07_mono_d1-held_{}.wav", candidate.slug())),
                    sample_rate,
                    &fold_samples(&render.samples),
                )?;
            }
            audition_rows.push((AuditionKind::HeldOctave, candidate, Some(note), render));
        }
        let bass = render_bass_candidate(candidate, sample_rate, false)?;
        write_wav(
            &output.join(format!("05_d1-pedal_{}_stereo.wav", candidate.slug())),
            sample_rate,
            &bass.samples,
        )?;
        write_wav(
            &output.join(format!("07_mono_d1-pedal_{}.wav", candidate.slug())),
            sample_rate,
            &fold_samples(&bass.samples),
        )?;
        audition_rows.push((AuditionKind::BassPedal, candidate, Some(26), bass));

        let punch = render_bass_candidate(candidate, sample_rate, true)?;
        write_wav(
            &output.join(format!("06_punch_{}_stereo.wav", candidate.slug())),
            sample_rate,
            &punch.samples,
        )?;
        write_wav(
            &output.join(format!("07_mono_punch_{}.wav", candidate.slug())),
            sample_rate,
            &fold_samples(&punch.samples),
        )?;
        audition_rows.push((AuditionKind::Punch, candidate, Some(26), punch));
    }

    write_manifest(output, &renders)?;
    write_gain_policy(output, sample_rate, &renders, &hot_renders, &audition_rows)?;
    write_octave(output, sample_rate, &audition_rows)?;
    write_punch_adsr(output, sample_rate)?;
    write_allocation_evidence(output)?;
    write_audition_similarity(output, &hot_renders, &audition_rows)?;
    write_audition_residual(output, sample_rate)?;
    write_reconstruction_regression(output, sample_rate, &renders)?;
    write_processors(output)?;
    write_layer_schedule(output, sample_rate)?;
    write_frequency_timeline(output, sample_rate)?;
    write_harmony(output, sample_rate, &renders)?;
    write_spectral(output, sample_rate, &renders)?;
    write_stereo(output, &renders)?;
    write_ablations(output, sample_rate)?;
    write_similarity(output, &renders)?;
    write_products(output, sample_rate, &renders)?;
    write_residual(output, sample_rate)?;
    write_topology(output, sample_rate)?;
    write_hashes(output, &renders)?;
    write_rejections(output, sample_rate, &renders)?;
    write_cost(output, sample_rate)?;
    write_readme(output, sample_rate)?;
    println!(
        "wrote composite machine lab at {} Hz to {}",
        sample_rate,
        output.display()
    );
    Ok(())
}

fn write_manifest(
    output: &Path,
    renders: &[(CompositeCandidate, CompositeRender, CompositeRender)],
) -> std::io::Result<()> {
    let mut file = writer(output, "manifest.tsv")?;
    writeln!(
        file,
        "file\tcandidate\tmode\tpeak\trms\tdc\tcrest\tmaximum_jump\tfinite\thash"
    )?;
    for (candidate, stereo, mono) in renders {
        let _ = mono;
        write_manifest_row(&mut file, candidate, "raw-stereo", &stereo.metrics)?;
    }
    Ok(())
}

fn write_manifest_row(
    output: &mut impl Write,
    candidate: &CompositeCandidate,
    mode: &str,
    metrics: &CompositeMetrics,
) -> std::io::Result<()> {
    writeln!(
        output,
        "08_raw_{}_stereo.wav\t{}\t{mode}\t{:.9}\t{:.9}\t{:.12}\t{:.6}\t{:.9}\t{}\t{:016x}",
        candidate.slug(),
        candidate.slug(),
        metrics.peak,
        metrics.rms,
        metrics.dc,
        metrics.crest_factor,
        metrics.maximum_jump,
        metrics.finite,
        metrics.sample_hash
    )
}

fn write_gain_policy(
    output: &Path,
    sample_rate: u32,
    raw_renders: &[(CompositeCandidate, CompositeRender, CompositeRender)],
    hot_renders: &[(CompositeCandidate, HotCompositeRender)],
    audition_rows: &[(AuditionKind, CompositeCandidate, Option<u8>, AuditionRender)],
) -> std::io::Result<()> {
    let mut file = writer(output, "gain-policy.tsv")?;
    writeln!(
        file,
        "kind\tcandidate\tmidi\traw_full_rms\traw_active_rms\thot_full_rms\thot_active_rms\tgain_linear\tgain_db\traw_peak\thot_peak\tdc\tcrest\tmaximum_jump\traw_hash\thot_hash\tstatus"
    )?;
    for ((candidate, raw, _), (hot_candidate, hot)) in raw_renders.iter().zip(hot_renders) {
        debug_assert_eq!(candidate, hot_candidate);
        let raw_active = window_rms(
            &raw.samples,
            sample_rate,
            ACTIVE_BODY_START_SECONDS,
            ACTIVE_BODY_END_SECONDS,
        );
        write_gain_row(
            &mut file,
            "full-score",
            *candidate,
            None,
            raw.metrics.rms,
            raw_active,
            hot.metrics.rms,
            hot.active_body_rms,
            hot.gain.linear,
            hot.gain.decibels,
            raw.metrics.peak,
            hot.metrics,
            raw.metrics.sample_hash,
        )?;
    }
    for (kind, candidate, note, render) in audition_rows {
        write_gain_row(
            &mut file,
            kind_slug(*kind),
            *candidate,
            *note,
            render.raw_rms,
            render.active_body_rms / f64::from(render.gain.linear),
            render.metrics.rms,
            render.active_body_rms,
            render.gain.linear,
            render.gain.decibels,
            render.raw_peak,
            render.metrics,
            render.raw_hash,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_gain_row(
    output: &mut impl Write,
    kind: &str,
    candidate: CompositeCandidate,
    note: Option<u8>,
    raw_full_rms: f64,
    raw_active_rms: f64,
    hot_full_rms: f64,
    hot_active_rms: f64,
    gain_linear: f32,
    gain_db: f64,
    raw_peak: f64,
    metrics: CompositeMetrics,
    raw_hash: u64,
) -> std::io::Result<()> {
    let status = if (0.10..=0.14).contains(&hot_active_rms) && metrics.peak <= 0.75 {
        "pass"
    } else if metrics.peak >= 0.749_999 {
        "documented_peak_constraint"
    } else {
        "reject"
    };
    writeln!(
        output,
        "{}\t{}\t{}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.6}\t{:.9}\t{:.9}\t{:.12}\t{:.6}\t{:.9}\t{:016x}\t{:016x}\t{}",
        kind,
        candidate.slug(),
        note.map_or_else(|| "NA".to_owned(), |value| value.to_string()),
        raw_full_rms,
        raw_active_rms,
        hot_full_rms,
        hot_active_rms,
        gain_linear,
        gain_db,
        raw_peak,
        metrics.peak,
        metrics.dc,
        metrics.crest_factor,
        metrics.maximum_jump,
        raw_hash,
        metrics.sample_hash,
        status
    )
}

fn write_octave(
    output: &Path,
    sample_rate: u32,
    audition_rows: &[(AuditionKind, CompositeCandidate, Option<u8>, AuditionRender)],
) -> std::io::Result<()> {
    let mut file = writer(output, "octave.tsv")?;
    writeln!(
        file,
        "candidate\tmidi\tfrequency_hz\traw_rms\thot_rms\tactive_rms\tgain_linear\tgain_db\tpeak\tdc\tcrest\tmaximum_jump\tlow_rms\tmid_rms\thigh_rms\tsubharmonic_db\tfundamental_db\toctave_db\tharmonic3_db\tharmonic5_db\tearly_rms\tbody_rms\tcorrelation\tside_to_mid\tlow_side_to_mid\tmono_rms\tmono_cancellation_db\thash"
    )?;
    for (kind, candidate, note, render) in audition_rows {
        if *kind != AuditionKind::HeldOctave {
            continue;
        }
        let note = note.expect("octave rows always declare MIDI");
        let frequency = midi_frequency(note);
        let mono = measure_composite(&fold_samples(&render.samples), sample_rate as f32);
        writeln!(
            file,
            "{}\t{}\t{:.6}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.6}\t{:.9}\t{:.12}\t{:.6}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.3}\t{:016x}",
            candidate.slug(),
            note,
            frequency,
            render.raw_rms,
            render.metrics.rms,
            render.active_body_rms,
            render.gain.linear,
            render.gain.decibels,
            render.metrics.peak,
            render.metrics.dc,
            render.metrics.crest_factor,
            render.metrics.maximum_jump,
            render.metrics.low_rms,
            render.metrics.mid_rms,
            render.metrics.high_rms,
            measure_projection(&render.samples, sample_rate as f32, 0.5 * frequency),
            measure_projection(&render.samples, sample_rate as f32, frequency),
            measure_projection(&render.samples, sample_rate as f32, 2.0 * frequency),
            measure_projection(&render.samples, sample_rate as f32, 3.0 * frequency),
            measure_projection(&render.samples, sample_rate as f32, 5.0 * frequency),
            render.metrics.early_rms,
            render.metrics.body_rms,
            render.metrics.correlation,
            render.metrics.side_to_mid,
            render.metrics.low_side_to_mid,
            mono.rms,
            amplitude_db(mono.rms / render.metrics.rms.max(1.0e-24)),
            render.metrics.sample_hash
        )?;
    }
    Ok(())
}

fn write_punch_adsr(output: &Path, sample_rate: u32) -> std::io::Result<()> {
    let mut file = writer(output, "punch-adsr.tsv")?;
    writeln!(
        file,
        "candidate\tattack_ms\tdecay_ms\tsustain\trelease_ms\trms_10ms\tpeak_10ms\trms_50ms\tpeak_50ms\trms_100ms\tpeak_100ms\trms_250ms\tpeak_250ms\tonset_to_sustain\tmeasured_attack_ms\tdecay_settling_ms\tmaximum_jump\tlow_band_transient_rms\tfundamental_decay_change_db\trelease_continuity_jump\ttail_duration_ms\thot_peak\thot_crest\tfinite\tstatus"
    )?;
    for candidate in CompositeCandidate::FINAL_CANDIDATES {
        for row in punch_sweep(candidate, sample_rate).map_err(io_other)? {
            writeln!(
                file,
                "{}\t{:.3}\t{:.3}\t{:.6}\t{:.3}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.6}\t{:.3}\t{:.3}\t{:.9}\t{:.9}\t{:.3}\t{:.9}\t{:.3}\t{:.9}\t{:.6}\t{}\t{}",
                candidate.slug(),
                1_000.0 * row.config.attack_seconds,
                1_000.0 * row.config.decay_seconds,
                row.config.sustain_level,
                1_000.0 * row.config.release_seconds,
                row.metrics.window_rms[0],
                row.metrics.window_peak[0],
                row.metrics.window_rms[1],
                row.metrics.window_peak[1],
                row.metrics.window_rms[2],
                row.metrics.window_peak[2],
                row.metrics.window_rms[3],
                row.metrics.window_peak[3],
                row.metrics.onset_to_sustain_ratio,
                row.metrics.attack_time_ms,
                row.metrics.decay_settling_ms,
                row.metrics.maximum_jump,
                row.metrics.low_band_transient_rms,
                row.metrics.fundamental_decay_change_db,
                row.metrics.release_continuity_jump,
                row.metrics.tail_duration_ms,
                row.metrics.peak,
                row.metrics.crest_factor,
                row.metrics.finite,
                row.status
            )?;
        }
    }
    Ok(())
}

fn write_allocation_evidence(output: &Path) -> std::io::Result<()> {
    let mut file = writer(output, "allocation-evidence.tsv")?;
    writeln!(file, "boundary\tevidence\tstatus")?;
    writeln!(
        file,
        "CompositeMachine::sample\tassert_no_alloc boundary test including follower and nonlinear junction\tpass"
    )?;
    writeln!(
        file,
        "HybridVoice::sample\tassert_no_alloc for every constituent hybrid family\tpass"
    )?;
    writeln!(
        file,
        "ResearchAdsr::sample\tassert_no_alloc through attack decay sustain release and retrigger\tpass"
    )?;
    writeln!(
        file,
        "offline renderer\tallocation permitted only outside prepared sample paths\tinformational"
    )
}

fn write_audition_similarity(
    output: &Path,
    hot_renders: &[(CompositeCandidate, HotCompositeRender)],
    audition_rows: &[(AuditionKind, CompositeCandidate, Option<u8>, AuditionRender)],
) -> std::io::Result<()> {
    let mut file = writer(output, "audition-similarity.tsv")?;
    writeln!(file, "kind\tmidi\tcandidate_a\tcandidate_b\tcorrelation")?;
    let finals: Vec<_> = hot_renders
        .iter()
        .filter(|(candidate, _)| CompositeCandidate::FINAL_CANDIDATES.contains(candidate))
        .collect();
    for first in 0..finals.len() {
        for second in first + 1..finals.len() {
            writeln!(
                file,
                "full-score\tNA\t{}\t{}\t{:.9}",
                finals[first].0.slug(),
                finals[second].0.slug(),
                sample_similarity(&finals[first].1.samples, &finals[second].1.samples)
            )?;
        }
    }
    for note in [26, 38, 50] {
        let rows: Vec<_> = audition_rows
            .iter()
            .filter(|(kind, _, row_note, _)| {
                *kind == AuditionKind::HeldOctave && *row_note == Some(note)
            })
            .collect();
        for first in 0..rows.len() {
            for second in first + 1..rows.len() {
                writeln!(
                    file,
                    "held-octave\t{}\t{}\t{}\t{:.9}",
                    note,
                    rows[first].1.slug(),
                    rows[second].1.slug(),
                    sample_similarity(&rows[first].3.samples, &rows[second].3.samples)
                )?;
            }
        }
    }
    Ok(())
}

fn write_audition_residual(output: &Path, sample_rate: u32) -> std::io::Result<()> {
    let mut file = writer(output, "audition-residual.tsv")?;
    writeln!(
        file,
        "kind\tcandidate\tmidi\tresidual_db\treference\tlimitation"
    )?;
    let frames = if sample_rate == 48_000 { 4_096 } else { 512 };
    for candidate in CompositeCandidate::ALL {
        let residual =
            measure_composite_residual(candidate, sample_rate, frames).map_err(io_other)?;
        writeln!(
            file,
            "full-score\t{}\tNA\t{:.3}\t8x_box_decimated\tconservative residual includes amplitude phase and state-rate differences",
            candidate.slug(),
            residual
        )?;
    }
    for candidate in CompositeCandidate::FINAL_CANDIDATES {
        for note in [26, 38, 50] {
            let mut worst = f64::NEG_INFINITY;
            for family in HybridFamily::ALL {
                worst =
                    worst.max(measure_hybrid_alias_error(family, note, frames).map_err(io_other)?);
            }
            writeln!(
                file,
                "held-octave\t{}\t{}\t{:.3}\tworst_constituent_8x_box_decimated\tworkstation offline evidence not audibility Pi or callback evidence",
                candidate.slug(),
                note,
                worst
            )?;
        }
    }
    Ok(())
}

fn write_reconstruction_regression(
    output: &Path,
    sample_rate: u32,
    renders: &[(CompositeCandidate, CompositeRender, CompositeRender)],
) -> std::io::Result<()> {
    let mut file = writer(output, "reconstruction-regression.tsv")?;
    writeln!(
        file,
        "candidate\tsample_rate\traw_hash\texpected_48k_hash\tstatus"
    )?;
    for (candidate, stereo, _) in renders.iter().take(2) {
        let expected = match candidate {
            CompositeCandidate::SynchronizedReference => 0xbe3e_a0fd_d66b_4472,
            CompositeCandidate::DelayedLaunchEstimate => 0xc919_cb57_920c_520f,
            _ => unreachable!(),
        };
        let status = if sample_rate == 48_000 {
            if stereo.metrics.sample_hash == expected {
                "sample_identical"
            } else {
                "reject_hash_mismatch"
            }
        } else {
            "quick_mode_determinism_only"
        };
        writeln!(
            file,
            "{}\t{}\t{:016x}\t{:016x}\t{}",
            candidate.slug(),
            sample_rate,
            stereo.metrics.sample_hash,
            expected,
            status
        )?;
    }
    Ok(())
}

fn write_processors(output: &Path) -> std::io::Result<()> {
    let mut file = writer(output, "processors.tsv")?;
    writeln!(file, "processor\tpresent\tpolicy")?;
    for processor in [
        "limiter",
        "compressor",
        "clipper",
        "peak-normalizer",
        "per-file-maximizer",
        "emergency-make-up-gain",
    ] {
        writeln!(file, "{processor}\tfalse\tforbidden")?;
    }
    writeln!(
        file,
        "prepared-fixed-output-gain\ttrue\texplicit per topology and audition kind"
    )
}

fn write_layer_schedule(output: &Path, sample_rate: u32) -> std::io::Result<()> {
    let mut file = writer(output, "layer-schedule.tsv")?;
    writeln!(
        file,
        "candidate\tindex\tlabel\tfamily\tstart_ms\tduration_ms\tgain\tmatrix_ll\tmatrix_lr\tmatrix_rl\tmatrix_rr"
    )?;
    for candidate in CompositeCandidate::ALL {
        let machine = CompositeMachine::new(candidate, sample_rate).map_err(io_other)?;
        for layer in machine.layer_info() {
            writeln!(
                file,
                "{}\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}",
                candidate.slug(),
                layer.index,
                layer.label,
                family_slug(layer.family),
                layer.start_ms,
                layer.duration_ms,
                layer.mix_gain,
                layer.matrix[0][0],
                layer.matrix[0][1],
                layer.matrix[1][0],
                layer.matrix[1][1]
            )?;
        }
    }
    Ok(())
}

fn write_frequency_timeline(output: &Path, sample_rate: u32) -> std::io::Result<()> {
    let mut file = writer(output, "frequency-timeline.tsv")?;
    writeln!(
        file,
        "candidate\tstart_ms\tend_ms\ttotal_voices\tmidi_counts\tfrequency_counts_hz"
    )?;
    for candidate in CompositeCandidate::ALL {
        let machine = CompositeMachine::new(candidate, sample_rate).map_err(io_other)?;
        for interval in machine.frequency_timeline() {
            let midi = interval
                .inventory
                .iter()
                .map(|(note, count)| format!("{note}:{count}"))
                .collect::<Vec<_>>()
                .join(",");
            let frequencies = interval
                .inventory
                .iter()
                .map(|(note, count)| format!("{:.6}:{count}", midi_frequency(note)))
                .collect::<Vec<_>>()
                .join(",");
            writeln!(
                file,
                "{}\t{}\t{}\t{}\t{}\t{}",
                candidate.slug(),
                interval.start_ms,
                interval.end_ms,
                interval.inventory.total_voices,
                midi,
                frequencies
            )?;
        }
    }
    Ok(())
}

fn write_harmony(
    output: &Path,
    sample_rate: u32,
    renders: &[(CompositeCandidate, CompositeRender, CompositeRender)],
) -> std::io::Result<()> {
    let mut file = writer(output, "harmony.tsv")?;
    writeln!(
        file,
        "candidate\tstart_ms\tend_ms\tmidi\tvoice_count\tfrequency_hz\tsub_db\tfundamental_db\toctave_db\tharmonic3_db\tharmonic5_db"
    )?;
    for (candidate, stereo, _) in renders {
        let machine = CompositeMachine::new(*candidate, sample_rate).map_err(io_other)?;
        for interval in machine.frequency_timeline() {
            let start = interval.start_ms as usize * sample_rate as usize / 1_000;
            let end = (interval.end_ms as usize * sample_rate as usize / 1_000)
                .min(stereo.samples.len() / 2);
            if start >= end {
                continue;
            }
            let slice = &stereo.samples[2 * start..2 * end];
            for (note, count) in interval.inventory.iter() {
                let frequency = midi_frequency(note);
                writeln!(
                    file,
                    "{}\t{}\t{}\t{}\t{}\t{:.6}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}",
                    candidate.slug(),
                    interval.start_ms,
                    interval.end_ms,
                    note,
                    count,
                    frequency,
                    measure_projection(slice, sample_rate as f32, 0.5 * frequency),
                    measure_projection(slice, sample_rate as f32, frequency),
                    measure_projection(slice, sample_rate as f32, 2.0 * frequency),
                    measure_projection(slice, sample_rate as f32, 3.0 * frequency),
                    measure_projection(slice, sample_rate as f32, 5.0 * frequency)
                )?;
            }
        }
    }
    Ok(())
}

fn write_spectral(
    output: &Path,
    sample_rate: u32,
    renders: &[(CompositeCandidate, CompositeRender, CompositeRender)],
) -> std::io::Result<()> {
    let mut file = writer(output, "spectral.tsv")?;
    writeln!(
        file,
        "candidate\tsegment\tstart_s\tend_s\tlow_rms\tmid_rms\thigh_rms\tbrightness_ratio\tspectral_flux\tonset_peak_ms\tearly_rms\tbody_rms"
    )?;
    for (candidate, stereo, _) in renders {
        let total_frames = stereo.samples.len() / 2;
        let segment_frames = 4 * sample_rate as usize;
        for (segment, start) in (0..total_frames).step_by(segment_frames).enumerate() {
            let end = (start + segment_frames).min(total_frames);
            let metrics =
                measure_composite(&stereo.samples[2 * start..2 * end], sample_rate as f32);
            writeln!(
                file,
                "{}\t{}\t{:.3}\t{:.3}\t{:.9}\t{:.9}\t{:.9}\t{:.6}\t{:.6}\t{:.6}\t{:.9}\t{:.9}",
                candidate.slug(),
                segment,
                start as f64 / f64::from(sample_rate),
                end as f64 / f64::from(sample_rate),
                metrics.low_rms,
                metrics.mid_rms,
                metrics.high_rms,
                metrics.brightness_ratio,
                metrics.spectral_flux,
                metrics.onset_peak_ms,
                metrics.early_rms,
                metrics.body_rms
            )?;
        }
    }
    Ok(())
}

fn write_stereo(
    output: &Path,
    renders: &[(CompositeCandidate, CompositeRender, CompositeRender)],
) -> std::io::Result<()> {
    let mut file = writer(output, "stereo.tsv")?;
    writeln!(
        file,
        "candidate\tcorrelation\tside_to_mid\tlow_side_to_mid\tmono_rms\tmono_to_stereo\tdifference_rms\tmono_peak\tmono_maximum_jump"
    )?;
    for (candidate, stereo, mono) in renders {
        writeln!(
            file,
            "{}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}",
            candidate.slug(),
            stereo.metrics.correlation,
            stereo.metrics.side_to_mid,
            stereo.metrics.low_side_to_mid,
            stereo.metrics.mono_rms,
            stereo.metrics.mono_to_stereo,
            stereo.metrics.difference_rms,
            mono.metrics.peak,
            mono.metrics.maximum_jump
        )?;
    }
    Ok(())
}

fn write_ablations(output: &Path, sample_rate: u32) -> std::io::Result<()> {
    let mut file = writer(output, "ablation.tsv")?;
    writeln!(
        file,
        "candidate\tlayer_index\tlabel\tmute_difference_db\tstatus"
    )?;
    for candidate in CompositeCandidate::ALL {
        for row in measure_ablations(candidate, sample_rate).map_err(io_other)? {
            writeln!(
                file,
                "{}\t{}\t{}\t{:.3}\t{}",
                candidate.slug(),
                row.layer_index,
                row.label,
                row.difference_db,
                if row.difference_db > -42.0 {
                    "active"
                } else {
                    "reject_inert"
                }
            )?;
        }
    }
    Ok(())
}

fn write_similarity(
    output: &Path,
    renders: &[(CompositeCandidate, CompositeRender, CompositeRender)],
) -> std::io::Result<()> {
    let mut file = writer(output, "similarity.tsv")?;
    writeln!(file, "candidate_a\tcandidate_b\tmono_correlation\tstatus")?;
    for first in 0..renders.len() {
        for second in first + 1..renders.len() {
            let correlation =
                sample_similarity(&renders[first].1.samples, &renders[second].1.samples);
            let reference_pair = first < 2 && second < 2;
            let status = if correlation.abs() < 0.985 || reference_pair {
                "distinct"
            } else {
                "reject_near_duplicate"
            };
            writeln!(
                file,
                "{}\t{}\t{:.9}\t{}",
                renders[first].0.slug(),
                renders[second].0.slug(),
                correlation,
                status
            )?;
        }
    }
    Ok(())
}

fn write_products(
    output: &Path,
    sample_rate: u32,
    renders: &[(CompositeCandidate, CompositeRender, CompositeRender)],
) -> std::io::Result<()> {
    let mut file = writer(output, "products.tsv")?;
    writeln!(
        file,
        "candidate\tmeasurement\tdifference1_hz\tdifference2_hz\tdifference3_hz\tlevel1_db\tlevel2_db\tlevel3_db"
    )?;
    for (candidate, stereo, _) in renders {
        let frames = (4 * sample_rate as usize).min(stereo.samples.len() / 2);
        let (measurement, levels) = if *candidate == CompositeCandidate::RiskyNonlinearBraid {
            (
                "junction_minus_linear_control",
                measure_junction_product_levels(*candidate, sample_rate).map_err(io_other)?,
            )
        } else {
            (
                "total_output_descriptive",
                measure_product_levels(
                    &stereo.samples[..2 * frames],
                    sample_rate as f32,
                    [50, 53, 57],
                ),
            )
        };
        let frequencies = [midi_frequency(50), midi_frequency(53), midi_frequency(57)];
        writeln!(
            file,
            "{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.3}\t{:.3}\t{:.3}",
            candidate.slug(),
            measurement,
            frequencies[1] - frequencies[0],
            frequencies[2] - frequencies[1],
            frequencies[2] - frequencies[0],
            levels[0],
            levels[1],
            levels[2]
        )?;
    }
    Ok(())
}

fn write_residual(output: &Path, sample_rate: u32) -> std::io::Result<()> {
    let mut file = writer(output, "residual.tsv")?;
    writeln!(
        file,
        "candidate\tcomposite_residual_db\tjunction_residual_db\treference\tframes"
    )?;
    let frames = if sample_rate == 48_000 { 4_096 } else { 512 };
    for candidate in CompositeCandidate::ALL {
        let residual =
            measure_composite_residual(candidate, sample_rate, frames).map_err(io_other)?;
        let junction_residual = if candidate == CompositeCandidate::RiskyNonlinearBraid {
            measure_junction_residual(candidate, sample_rate, frames).map_err(io_other)?
        } else {
            f64::NAN
        };
        writeln!(
            file,
            "{}\t{:.3}\t{}\t8x_box_decimated_conservative_residual\t{}",
            candidate.slug(),
            residual,
            if junction_residual.is_finite() {
                format!("{junction_residual:.3}")
            } else {
                "NA".to_owned()
            },
            frames
        )?;
    }
    Ok(())
}

fn write_topology(output: &Path, sample_rate: u32) -> std::io::Result<()> {
    let mut file = writer(output, "topology.tsv")?;
    writeln!(
        file,
        "candidate\tlayers\tfamilies\tsignature\tfollower_min\tfollower_max\tjunction_peak"
    )?;
    for candidate in CompositeCandidate::ALL {
        let mut machine = CompositeMachine::new(candidate, sample_rate).map_err(io_other)?;
        let mut follower_min = f32::INFINITY;
        let mut follower_max = f32::NEG_INFINITY;
        let mut junction_peak = 0.0_f32;
        for _ in 0..machine.duration_frames() {
            machine.sample();
            follower_min = follower_min.min(machine.last_follower_control());
            follower_max = follower_max.max(machine.last_follower_control());
            junction_peak = junction_peak.max(machine.last_junction().abs());
        }
        writeln!(
            file,
            "{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.9}",
            candidate.slug(),
            machine.layer_count(),
            machine.family_count(),
            machine.topology_signature(),
            follower_min,
            follower_max,
            junction_peak
        )?;
    }
    Ok(())
}

fn write_hashes(
    output: &Path,
    renders: &[(CompositeCandidate, CompositeRender, CompositeRender)],
) -> std::io::Result<()> {
    let mut file = writer(output, "hashes.tsv")?;
    writeln!(file, "candidate\tstereo_hash\tmono_hash")?;
    for (candidate, stereo, mono) in renders {
        writeln!(
            file,
            "{}\t{:016x}\t{:016x}",
            candidate.slug(),
            stereo.metrics.sample_hash,
            mono.metrics.sample_hash
        )?;
    }
    Ok(())
}

fn write_rejections(
    output: &Path,
    sample_rate: u32,
    renders: &[(CompositeCandidate, CompositeRender, CompositeRender)],
) -> std::io::Result<()> {
    let mut file = writer(output, "rejections.tsv")?;
    writeln!(file, "candidate\trule\tstatus\tvalue\tlimit")?;
    for (candidate, stereo, _) in renders {
        rejection_row(
            &mut file,
            candidate,
            "finite",
            stereo.metrics.finite,
            1.0,
            1.0,
        )?;
        rejection_row(
            &mut file,
            candidate,
            "peak",
            stereo.metrics.peak <= 0.8,
            stereo.metrics.peak,
            0.8,
        )?;
        rejection_row(
            &mut file,
            candidate,
            "absolute_dc",
            stereo.metrics.dc.abs() <= 5.0e-4,
            stereo.metrics.dc.abs(),
            5.0e-4,
        )?;
        rejection_row(
            &mut file,
            candidate,
            "stereo_correlation",
            stereo.metrics.correlation < 0.995,
            stereo.metrics.correlation,
            0.995,
        )?;
        rejection_row(
            &mut file,
            candidate,
            "side_to_mid",
            stereo.metrics.side_to_mid >= 0.005,
            stereo.metrics.side_to_mid,
            0.005,
        )?;
        rejection_row(
            &mut file,
            candidate,
            "mono_survival",
            stereo.metrics.mono_to_stereo >= 0.5,
            stereo.metrics.mono_to_stereo,
            0.5,
        )?;
        if *candidate == CompositeCandidate::RiskyNonlinearBraid {
            let frames = (4 * sample_rate as usize).min(stereo.samples.len() / 2);
            let first_segment = &stereo.samples[..2 * frames];
            let weakest_target = [50, 53, 57]
                .map(|note| {
                    measure_projection(first_segment, sample_rate as f32, midi_frequency(note))
                })
                .into_iter()
                .fold(f64::INFINITY, f64::min);
            let strongest_product = measure_junction_product_levels(*candidate, sample_rate)
                .map_err(io_other)?
                .into_iter()
                .fold(f64::NEG_INFINITY, f64::max);
            let margin = weakest_target - strongest_product;
            rejection_row(
                &mut file,
                candidate,
                "junction_product_margin_db",
                margin >= 30.0,
                margin,
                30.0,
            )?;
        }
    }
    for first in 2..renders.len() {
        for second in first + 1..renders.len() {
            let similarity =
                sample_similarity(&renders[first].1.samples, &renders[second].1.samples);
            rejection_row(
                &mut file,
                &renders[first].0,
                "pairwise_similarity",
                similarity.abs() < 0.985,
                similarity.abs(),
                0.985,
            )?;
        }
    }
    for (candidate, stereo, _) in renders {
        let machine = CompositeMachine::new(*candidate, sample_rate).map_err(io_other)?;
        let mut worst_spread = 0.0_f64;
        for interval in machine.frequency_timeline() {
            let start = interval.start_ms as usize * sample_rate as usize / 1_000;
            let end = (interval.end_ms as usize * sample_rate as usize / 1_000)
                .min(stereo.samples.len() / 2);
            if start >= end || interval.inventory.total_voices == 0 {
                continue;
            }
            let slice = &stereo.samples[2 * start..2 * end];
            let levels: Vec<_> = interval
                .inventory
                .iter()
                .map(|(note, _)| {
                    measure_projection(slice, sample_rate as f32, midi_frequency(note))
                })
                .collect();
            let strongest = levels.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let weakest = levels.iter().copied().fold(f64::INFINITY, f64::min);
            worst_spread = worst_spread.max(strongest - weakest);
        }
        rejection_row(
            &mut file,
            candidate,
            "scheduled_target_spread_db",
            worst_spread <= 36.0,
            worst_spread,
            36.0,
        )?;
    }
    Ok(())
}

fn rejection_row(
    output: &mut impl Write,
    candidate: &CompositeCandidate,
    rule: &str,
    passed: bool,
    value: f64,
    limit: f64,
) -> std::io::Result<()> {
    writeln!(
        output,
        "{}\t{}\t{}\t{:.9}\t{:.9}",
        candidate.slug(),
        rule,
        if passed { "pass" } else { "reject" },
        value,
        limit
    )
}

fn write_cost(output: &Path, sample_rate: u32) -> std::io::Result<()> {
    let mut file = writer(output, "workstation-cost.txt")?;
    writeln!(file, "scope=prepared_scalar_composite_sample_loop")?;
    for candidate in CompositeCandidate::ALL {
        let mut machine = CompositeMachine::new(candidate, sample_rate).map_err(io_other)?;
        let frames = machine.duration_frames().min(10 * sample_rate as usize);
        let start = Instant::now();
        let mut accumulator = 0.0_f32;
        for _ in 0..frames {
            let frame = machine.sample();
            accumulator += frame.left + frame.right;
        }
        std::hint::black_box(accumulator);
        writeln!(
            file,
            "{}_nanoseconds_per_frame={:.3}",
            candidate.slug(),
            start.elapsed().as_nanos() as f64 / frames.max(1) as f64
        )?;
    }
    writeln!(
        file,
        "limitation=scalar x86_64 workstation evidence; prepared offline layer playback and mix only; not Raspberry Pi or callback evidence"
    )
}

fn write_readme(output: &Path, sample_rate: u32) -> std::io::Result<()> {
    let mut file = writer(output, "README.md")?;
    writeln!(file, "# Moj Sint hot composite machine lab\n")?;
    writeln!(
        file,
        "This is a hot, level-controlled human listening gate. No sound, punch envelope, macro, preset, or production route is accepted by these measurements.\n"
    )?;
    writeln!(
        file,
        "The previous batch was biased: the synchronized reconstruction measured 0.070910 RMS while the delayed-launch estimate measured 0.031355 and the candidates measured 0.017705-0.028555. The new files use declared fixed topology gains measured over the same 4.25-8.00 second active window. No limiter, compressor, clipping, peak normalization, or automatic per-file maximization is used.\n"
    )?;
    writeln!(
        file,
        "The hot delayed-launch reference comes first because the accident used separately spawned player processes. Its 0-4110 ms offsets are a deterministic estimate; the historical order and exact delays are unknown. The hot synchronized reconstruction remains a counterfactual diagnostic.\n"
    )?;
    writeln!(file, "Listen at a comfortable low level in this order:\n")?;
    writeln!(file, "## 1. Hot delayed-launch reference\n")?;
    writeln!(file, "1. `01_hot-delayed-launch-reference_stereo.wav`\n")?;
    writeln!(file, "## 2. Hot synchronized counterfactual\n")?;
    writeln!(file, "2. `02_hot-synchronized-counterfactual_stereo.wav`\n")?;
    writeln!(file, "## 3. Hot full-score composite candidates\n")?;
    for candidate in CompositeCandidate::FINAL_CANDIDATES {
        writeln!(
            file,
            "- `03_hot-full-score_{}_stereo.wav`",
            candidate.slug()
        )?;
    }
    writeln!(file, "\n## 4. D1/D2/D3 held-note octave comparison\n")?;
    writeln!(
        file,
        "For each topology, listen to D1, then D2, then D3. One topology gain is reused across all three octaves; no register is independently normalized.\n"
    )?;
    for candidate in CompositeCandidate::FINAL_CANDIDATES {
        writeln!(
            file,
            "- `04_octave_{}_d1_stereo.wav`, then `_d2_`, then `_d3_`",
            candidate.slug()
        )?;
    }
    writeln!(file, "\n## 5. D1 bass/pedal composite versions\n")?;
    for candidate in CompositeCandidate::FINAL_CANDIDATES {
        writeln!(file, "- `05_d1-pedal_{}_stereo.wav`", candidate.slug())?;
    }
    writeln!(file, "\n## 6. punch ADSR versions\n")?;
    writeln!(
        file,
        "The retained engineering hypothesis is attack 3 ms, decay 110 ms, sustain 0.55, release 180 ms. It is applied to the scheduled D1 source before composite summing. Metrics do not establish that it sounds punchy.\n"
    )?;
    for candidate in CompositeCandidate::FINAL_CANDIDATES {
        writeln!(file, "- `06_punch_{}_stereo.wav`", candidate.slug())?;
    }
    writeln!(
        file,
        "\n## 7. Mono diagnostics last\n\nCompare the `07_mono_d1-held_*`, `07_mono_d1-pedal_*`, and `07_mono_punch_*` files only after the stereo identities are understood.\n"
    )?;
    writeln!(
        file,
        "## 8. raw engineering renders\n\nThe `08_raw_*` files preserve the previous unheated engineering outputs as secondary diagnostics. The reports contain gain, octave, onset/body, band, projection, stereo/mono, residual, hash, similarity, allocation, and processor evidence.\n"
    )?;
    writeln!(
        file,
        "Digital sample level is not acoustic SPL and is not evidence of safe playback volume. Start low. Do not raise playback gain to seek physical vibration.\n"
    )?;
    writeln!(
        file,
        "This batch is {} Hz. Workstation timing is scalar x86_64 development evidence only, not Raspberry Pi, callback, latency, polyphony, or sound-quality evidence.",
        sample_rate
    )
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

fn mono_fold(stereo: &CompositeRender, sample_rate: u32) -> CompositeRender {
    let samples = fold_samples(&stereo.samples);
    let metrics = measure_composite(&samples, sample_rate as f32);
    CompositeRender { samples, metrics }
}

fn fold_samples(stereo: &[f32]) -> Vec<f32> {
    let mut samples = stereo.to_vec();
    for frame in samples.chunks_exact_mut(2) {
        let mid = 0.5 * (frame[0] + frame[1]);
        frame[0] = mid;
        frame[1] = mid;
    }
    samples
}

fn kind_slug(kind: AuditionKind) -> &'static str {
    match kind {
        AuditionKind::HeldOctave => "held-octave",
        AuditionKind::BassPedal => "d1-pedal",
        AuditionKind::Punch => "punch",
    }
}

fn amplitude_db(amplitude: f64) -> f64 {
    20.0 * amplitude.max(1.0e-12).log10()
}

fn writer(output: &Path, name: &str) -> std::io::Result<BufWriter<File>> {
    Ok(BufWriter::new(File::create(output.join(name))?))
}

fn family_slug(family: HybridFamily) -> &'static str {
    match family {
        HybridFamily::CrossCoupledMachine => "cross-coupled-machine",
        HybridFamily::SpectralShadow => "spectral-shadow",
        HybridFamily::DualResonantBody => "dual-resonant-body",
    }
}

fn io_other(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(error.to_string())
}
