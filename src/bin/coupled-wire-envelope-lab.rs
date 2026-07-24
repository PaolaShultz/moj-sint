use moj_sint::coupled_wire_motion::{
    CoupledMotionEvidence, CoupledMotionProfile, CoupledWireEnvelopeVoice, DEVELOPED_DECAY_SCALE,
    MOTION_SPLIT_HZ, REFERENCE_GAIN, evaluate as evaluate_motion, preview_developed,
    render_preview as render_motion_preview, select_shared_gain,
};
use moj_sint::hybrid_subset::SubsetMetrics;
use moj_sint::struck_object::{
    StruckTopology, evaluate as evaluate_struck, preview as preview_struck,
    render_preview as render_struck_preview,
};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

const SAMPLE_RATE: u32 = 48_000;

struct LabEntry {
    profile: CoupledMotionProfile,
    samples: Vec<f32>,
    gain: f32,
    total_rms: f64,
    metrics: SubsetMetrics,
    tonal_pass_fraction: f64,
    spectral_flatness: f64,
    motion_evidence: Option<CoupledMotionEvidence>,
    reasons: Vec<&'static str>,
}

impl LabEntry {
    fn passes(&self) -> bool {
        self.reasons.is_empty()
    }
}

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = match args.as_slice() {
        [command, output] if command == "render" || command == "render-test" => {
            render_lab(Path::new(output))
        }
        _ => {
            eprintln!("Usage: coupled-wire-envelope-lab <render|render-test> <output-directory>");
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn render_lab(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output)?;
    let mut entries = Vec::with_capacity(4);

    let reference_preview = preview_struck(StruckTopology::CoupledWire, SAMPLE_RATE)?;
    let reference = render_struck_preview(&reference_preview, REFERENCE_GAIN)?;
    let reference_evidence = evaluate_struck(&reference);
    entries.push(LabEntry {
        profile: CoupledMotionProfile::Reference,
        samples: reference.samples,
        gain: reference.gain,
        total_rms: reference_evidence.total_rms,
        metrics: reference_evidence.metrics,
        tonal_pass_fraction: reference_evidence.tonal_pass_fraction,
        spectral_flatness: reference_evidence.spectral_flatness,
        motion_evidence: None,
        reasons: reference_evidence
            .rejection_reasons()
            .into_iter()
            .map(|reason| reason.slug())
            .collect(),
    });

    let previews = preview_developed(SAMPLE_RATE)?;
    let developed_gain = select_shared_gain(&previews)?;
    for preview in &previews {
        let render = render_motion_preview(preview, developed_gain)?;
        let evidence = evaluate_motion(&render);
        entries.push(LabEntry {
            profile: render.profile,
            samples: render.samples,
            gain: render.gain,
            total_rms: evidence.total_rms,
            metrics: evidence.metrics,
            tonal_pass_fraction: evidence.tonal_pass_fraction,
            spectral_flatness: evidence.spectral_flatness,
            motion_evidence: Some(evidence),
            reasons: evidence
                .rejection_reasons()
                .into_iter()
                .map(|reason| reason.slug())
                .collect(),
        });
    }

    write_manifest(output, &entries)?;
    write_envelopes(output)?;
    write_motion(output)?;
    write_metrics(output, &entries)?;
    write_rejections(output, &entries)?;
    write_hashes(output, &entries)?;
    write_summary(output, developed_gain, &entries)?;
    write_readme(output)?;
    write_cost(output)?;

    for entry in &entries {
        if entry.passes() {
            write_wav(&output.join(entry.profile.filename()), &entry.samples)?;
        }
    }
    let rejected = entries.iter().filter(|entry| !entry.passes()).count();
    if rejected != 0 {
        return Err(format!("{rejected} Coupled Wire files failed the gate").into());
    }
    Ok(())
}

fn write_manifest(output: &Path, entries: &[LabEntry]) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(output.join("manifest.tsv"))?);
    writeln!(
        file,
        "file\tprofile\tduration_ms\tmodal_decay_scale\tgain\tstatus"
    )?;
    for entry in entries {
        let decay_scale = if entry.profile == CoupledMotionProfile::Reference {
            1.0
        } else {
            DEVELOPED_DECAY_SCALE
        };
        writeln!(
            file,
            "{}\t{}\t{}\t{decay_scale:.3}\t{:.4}\t{}",
            entry.profile.filename(),
            entry.profile.slug(),
            entry.profile.spec().duration_ms,
            entry.gain,
            if entry.passes() { "pass" } else { "reject" }
        )?;
    }
    Ok(())
}

fn write_envelopes(output: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(output.join("envelopes.tsv"))?);
    writeln!(
        file,
        "profile\tattack_ms\tdecay_ms\tsustain\tnote_off_ms\trelease_ms\tduration_ms"
    )?;
    for profile in CoupledMotionProfile::ALL {
        let spec = profile.spec();
        writeln!(
            file,
            "{}\t{}\t{}\t{:.3}\t{}\t{}\t{}",
            profile.slug(),
            spec.attack_ms,
            spec.decay_ms,
            spec.sustain,
            spec.note_off_ms,
            spec.release_ms,
            spec.duration_ms
        )?;
    }
    Ok(())
}

fn write_motion(output: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(output.join("motion.tsv"))?);
    writeln!(
        file,
        "profile\tsplit_hz\tpan_rate_hz\tpan_depth\tmono_sum_policy"
    )?;
    for profile in CoupledMotionProfile::ALL {
        let spec = profile.spec();
        writeln!(
            file,
            "{}\t{MOTION_SPLIT_HZ:.1}\t{:.3}\t{:.3}\tequal_and_opposite",
            profile.slug(),
            spec.pan_rate_hz,
            spec.pan_depth
        )?;
    }
    Ok(())
}

fn write_metrics(output: &Path, entries: &[LabEntry]) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(output.join("metrics.tsv"))?);
    writeln!(
        file,
        "file\ttotal_rms\ttotal_rms_dbfs\tpeak\tdc\tmaximum_jump\tceiling_proportion\tcorrelation\tmono_loss_db\ttonal_pass_fraction\tspectral_flatness\tattack_0_60ms\tattack_80_140ms\tattack_160_220ms\tsustain_rms\trelease_early_rms\trelease_late_rms\tfinite\treturns_to_zero"
    )?;
    for entry in entries {
        let (attack_0, attack_1, attack_2, sustain, release_early, release_late, returns_to_zero) =
            match entry.motion_evidence {
                Some(evidence) => (
                    format!("{:.9}", evidence.attack_rms[0]),
                    format!("{:.9}", evidence.attack_rms[1]),
                    format!("{:.9}", evidence.attack_rms[2]),
                    format!("{:.9}", evidence.sustain_rms),
                    format!("{:.9}", evidence.release_early_rms),
                    format!("{:.9}", evidence.release_late_rms),
                    evidence.returns_to_zero.to_string(),
                ),
                None => (
                    "NA".into(),
                    "NA".into(),
                    "NA".into(),
                    "NA".into(),
                    "NA".into(),
                    "NA".into(),
                    "true".into(),
                ),
            };
        writeln!(
            file,
            "{}\t{:.9}\t{:.4}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.6}\t{:.6}\t{:.9}\t{attack_0}\t{attack_1}\t{attack_2}\t{sustain}\t{release_early}\t{release_late}\t{}\t{returns_to_zero}",
            entry.profile.filename(),
            entry.total_rms,
            linear_db(entry.total_rms),
            entry.metrics.peak,
            entry.metrics.dc,
            entry.metrics.maximum_jump,
            entry.metrics.ceiling_proportion,
            entry.metrics.correlation,
            entry.metrics.mono_loss_db,
            entry.tonal_pass_fraction,
            entry.spectral_flatness,
            entry.metrics.finite
        )?;
    }
    Ok(())
}

fn write_rejections(output: &Path, entries: &[LabEntry]) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(output.join("rejections.tsv"))?);
    writeln!(file, "profile\tfile\tstatus\treasons")?;
    for entry in entries {
        writeln!(
            file,
            "{}\t{}\t{}\t{}",
            entry.profile.slug(),
            entry.profile.filename(),
            if entry.passes() { "pass" } else { "reject" },
            if entry.reasons.is_empty() {
                "none".to_owned()
            } else {
                entry.reasons.join(",")
            }
        )?;
    }
    Ok(())
}

fn write_hashes(output: &Path, entries: &[LabEntry]) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(output.join("hashes.tsv"))?);
    writeln!(file, "file\tfnv1a_sample_hash")?;
    for entry in entries {
        writeln!(
            file,
            "{}\t{:016x}",
            entry.profile.filename(),
            entry.metrics.sample_hash
        )?;
    }
    Ok(())
}

fn write_summary(output: &Path, developed_gain: f32, entries: &[LabEntry]) -> std::io::Result<()> {
    let passed = entries.iter().filter(|entry| entry.passes()).count();
    let mut file = BufWriter::new(File::create(output.join("generation-summary.tsv"))?);
    writeln!(file, "field\tvalue")?;
    writeln!(file, "sample_rate\t{SAMPLE_RATE}")?;
    writeln!(file, "reference_gain\t{REFERENCE_GAIN:.4}")?;
    writeln!(file, "developed_shared_gain\t{developed_gain:.4}")?;
    writeln!(file, "modal_decay_scale\t{DEVELOPED_DECAY_SCALE:.3}")?;
    writeln!(file, "motion_split_hz\t{MOTION_SPLIT_HZ:.1}")?;
    writeln!(file, "candidates\t{}", entries.len())?;
    writeln!(file, "passed\t{passed}")?;
    writeln!(file, "rejected\t{}", entries.len() - passed)?;
    writeln!(file, "new_audio_nonlinearity\tfalse")?;
    Ok(())
}

fn write_readme(output: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(output.join("README.md"))?);
    writeln!(file, "# Coupled Wire envelope and motion\n")?;
    writeln!(
        file,
        "The first file is the exact accepted Coupled Wire reference. The next three retain its exciter, pitch, modal ratios, coupling, brightness structure, and pickup identity while extending resonant loss and applying one whole-sound envelope.\n"
    )?;
    writeln!(
        file,
        "Listen in numbered order. Warm Hold isolates the longer envelope. Slow High Orbit and Fast High Orbit add restrained equal-and-opposite movement only from the high residual above 320 Hz. The low body remains fixed and the mono sum is unchanged before sparse presentation limiting.\n"
    )?;
    writeln!(
        file,
        "The three developments share one fixed gain; no file is individually normalized. There is no reverb, delay, chorus, compressor, soft saturation, dry parallel layer, or pitch vibrato. Automated reports reject weak, noisy, unstable, over-limited, or mono-unsafe output; human listening decides whether the longer behavior serves the instrument."
    )
}

fn write_cost(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = BufWriter::new(File::create(output.join("workstation-cost.txt"))?);
    writeln!(file, "scope=isolated_scalar_coupled_wire_envelope_voice")?;
    for profile in CoupledMotionProfile::ALL {
        let mut voice = CoupledWireEnvelopeVoice::new(profile, SAMPLE_RATE)?;
        let frames = voice.duration_frames();
        let start = Instant::now();
        let mut accumulator = 0.0_f32;
        for _ in 0..frames {
            let frame = voice.sample();
            accumulator += frame.left + frame.right;
        }
        std::hint::black_box(accumulator);
        writeln!(
            file,
            "{}_nanoseconds_per_frame={:.3}",
            profile.slug(),
            start.elapsed().as_nanos() as f64 / frames as f64
        )?;
    }
    writeln!(
        file,
        "limitation=scalar x86_64 workstation generation evidence; not callback or Raspberry Pi evidence"
    )?;
    Ok(())
}

fn write_wav(path: &Path, samples: &[f32]) -> Result<(), hound::Error> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for sample in samples {
        writer.write_sample(*sample)?;
    }
    writer.finalize()
}

fn linear_db(value: f64) -> f64 {
    20.0 * value.max(1.0e-12).log10()
}
