use moj_sint::composite_machine::{
    CompositeCandidate, CompositeMachine, CompositeMetrics, CompositeRender, measure_ablations,
    measure_composite, measure_composite_residual, measure_junction_product_levels,
    measure_junction_residual, measure_product_levels, measure_projection, midi_frequency,
    render_composite, sample_similarity,
};
use moj_sint::dsp::hybrid::HybridFamily;
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
            &output.join(format!("{}_stereo.wav", candidate.slug())),
            sample_rate,
            &stereo.samples,
        )?;
        write_wav(
            &output.join(format!("{}_mono.wav", candidate.slug())),
            sample_rate,
            &mono.samples,
        )?;
        renders.push((candidate, stereo, mono));
    }

    write_manifest(output, &renders)?;
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
        write_manifest_row(&mut file, candidate, "stereo", &stereo.metrics)?;
        write_manifest_row(&mut file, candidate, "mono", &mono.metrics)?;
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
        "{}_{mode}.wav\t{}\t{mode}\t{:.9}\t{:.9}\t{:.12}\t{:.6}\t{:.9}\t{}\t{:016x}",
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
    writeln!(file, "# Moj Sint composite machine lab\n")?;
    writeln!(
        file,
        "This batch follows the first strongly positive Moj Sint listening accident. It remains an open human listening gate; no sound, macro, preset, or production route is accepted.\n"
    )?;
    writeln!(
        file,
        "The synchronized reference is a schedule-exact synchronized counterfactual: all twelve internally synthesized source schedules begin at sample zero. The real accident used separately spawned player processes, so that was not what happened.\n"
    )?;
    writeln!(
        file,
        "The delayed-launch estimate uses deterministic offsets from 0 to 4110 ms to model noticeable process-launch skew. The historical order and exact delays are unknown and cannot be recovered from the WAV files. Treat this as a controlled estimate, not an exact replay.\n"
    )?;
    writeln!(file, "Listen at a comfortable low level in this order:\n")?;
    for (index, candidate) in CompositeCandidate::ALL.iter().enumerate() {
        writeln!(file, "{}. `{}_stereo.wav`", index + 1, candidate.slug())?;
    }
    writeln!(
        file,
        "\nAfter those identities are understood, compare the matching `_mono.wav` diagnostics. The reports explain layer scheduling, frequency inventory, ablation, harmony/spines, spectrum, stereo/mono behavior, products, residuals, topology, hashes, similarity, and cost.\n"
    )?;
    writeln!(
        file,
        "No final limiter or peak maximizer is used. Prepared layer gains reserve headroom. Digital sample values do not report acoustic SPL, safe playback level, tactile sensation, or translation on a particular system. Start low and do not raise gain to seek physical vibration.\n"
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
    let mut samples = stereo.samples.clone();
    for frame in samples.chunks_exact_mut(2) {
        let mid = 0.5 * (frame[0] + frame[1]);
        frame[0] = mid;
        frame[1] = mid;
    }
    let metrics = measure_composite(&samples, sample_rate as f32);
    CompositeRender { samples, metrics }
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
