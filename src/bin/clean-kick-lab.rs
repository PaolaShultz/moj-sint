use moj_sint::clean_kick::{
    KickConfig, KickEvidence, KickRender, KickTopology, SAMPLE_RATE, evaluate, render_repeated,
    render_solo, select,
};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

const BPM: u32 = 124;
const BARS: u32 = 4;
const WAV_NAMES: [&str; 4] = [
    "01_house-impact_solo.wav",
    "02_long-pressure_solo.wav",
    "03_house-impact_124bpm.wav",
    "04_long-pressure_124bpm.wav",
];

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = match args.as_slice() {
        [command, output] if command == "render" || command == "render-test" => {
            render_lab(Path::new(output))
        }
        _ => {
            eprintln!("Usage: clean-kick-lab <render|render-test> <output-directory>");
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
    let started = Instant::now();
    require_empty_directory(output)?;

    let house_config = select(KickTopology::HouseImpact, SAMPLE_RATE)?;
    let pressure_config = select(KickTopology::LongPressure, SAMPLE_RATE)?;
    let house_solo = render_solo(house_config, SAMPLE_RATE)?;
    let pressure_solo = render_solo(pressure_config, SAMPLE_RATE)?;
    let house_evidence = evaluate(&house_solo)?;
    let pressure_evidence = evaluate(&pressure_solo)?;
    if !house_evidence.rejection_reasons().is_empty()
        || !pressure_evidence.rejection_reasons().is_empty()
    {
        return Err("a selected clean-kick voice failed re-evaluation".into());
    }

    let renders = [
        house_solo,
        pressure_solo,
        render_repeated(house_config, SAMPLE_RATE, BPM, BARS)?,
        render_repeated(pressure_config, SAMPLE_RATE, BPM, BARS)?,
    ];
    fs::create_dir_all(output)?;
    write_readme(output)?;
    write_settings(output, house_config, pressure_config, &renders)?;
    write_metrics(output, house_evidence, pressure_evidence)?;
    write_hashes(output, &renders)?;
    write_summary(output, house_config, pressure_config)?;
    write_cost(output, started.elapsed().as_secs_f64())?;
    for (name, render) in WAV_NAMES.into_iter().zip(&renders) {
        write_wav(&output.join(name), render)?;
    }
    Ok(())
}

fn require_empty_directory(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if output.exists() && fs::read_dir(output)?.next().is_some() {
        return Err(format!("output directory is not empty: {}", output.display()).into());
    }
    Ok(())
}

fn write_readme(output: &Path) -> std::io::Result<()> {
    let mut file = writer(output, "README.md")?;
    writeln!(file, "# Two clean house-kick candidates\n")?;
    writeln!(
        file,
        "Listen in filename order. Files 1-2 are isolated one-shots; files 3-4 are four bars of 124 BPM quarter-note retriggers plus the complete tail.\n"
    )?;
    writeln!(
        file,
        "House Impact is an envelope-driven phase-modulated oscillator. Long Pressure is a coupled impact/body modal source. They are structurally different voices, not parameter variations of one graph.\n"
    )?;
    writeln!(
        file,
        "No clipper, limiter, compressor, saturation, normalizer, or noise layer is present. Static gain and a linear 5 Hz DC blocker are the only presentation operations.\n"
    )?;
    writeln!(
        file,
        "Engineering checks reject broken output; whether either kick is massive, gut-ripping, clean-sounding, or house-ready is unresolved. The listening decision remains open."
    )
}

fn write_settings(
    output: &Path,
    house: KickConfig,
    pressure: KickConfig,
    renders: &[KickRender; 4],
) -> std::io::Result<()> {
    let mut file = writer(output, "settings.tsv")?;
    writeln!(
        file,
        "file\ttopology\tcontext\tsample_rate\tchannels\tformat\toutput_gain\tmodifier_amount\tbpm\tbars\tframes"
    )?;
    for (index, (name, render)) in WAV_NAMES.into_iter().zip(renders).enumerate() {
        let config = if index % 2 == 0 { house } else { pressure };
        let context = if index < 2 { "solo" } else { "quarter_note" };
        writeln!(
            file,
            "{name}\t{:?}\t{context}\t{}\t2\tfloat32\t{:.6}\t{:.6}\t{}\t{}\t{}",
            config.topology,
            render.sample_rate,
            config.output_gain,
            config.modifier_amount,
            if index < 2 { 0 } else { BPM },
            if index < 2 { 0 } else { BARS },
            render.samples.len()
        )?;
    }
    Ok(())
}

fn write_metrics(
    output: &Path,
    house: KickEvidence,
    pressure: KickEvidence,
) -> std::io::Result<()> {
    let mut file = writer(output, "metrics.tsv")?;
    writeln!(
        file,
        "topology\tsample_peak_dbfs\ttrue_peak_dbfs\trms_dbfs\tabsolute_dc\tmaximum_jump\tceiling_contacts\tfinite\ttail_is_zero\thigh_rate_residual_db\tonset_ablation_db\tbody_ablation_db"
    )?;
    for evidence in [house, pressure] {
        writeln!(
            file,
            "{:?}\t{:.6}\t{:.6}\t{:.6}\t{:.9}\t{:.9}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}",
            evidence.topology,
            evidence.metrics.sample_peak_dbfs,
            evidence.metrics.true_peak_dbfs,
            evidence.metrics.rms_dbfs,
            evidence.metrics.absolute_dc,
            evidence.metrics.maximum_jump,
            evidence.metrics.ceiling_contacts,
            evidence.metrics.finite,
            evidence.metrics.tail_is_zero,
            evidence.high_rate_residual_db,
            evidence.onset_ablation_db,
            evidence.body_ablation_db
        )?;
    }
    Ok(())
}

fn write_hashes(output: &Path, renders: &[KickRender; 4]) -> std::io::Result<()> {
    let mut file = writer(output, "hashes.tsv")?;
    writeln!(file, "file\tfnv1a_interleaved_stereo_sample_hash")?;
    for (name, render) in WAV_NAMES.into_iter().zip(renders) {
        writeln!(file, "{name}\t{:016x}", stereo_sample_hash(&render.samples))?;
    }
    Ok(())
}

fn write_summary(output: &Path, house: KickConfig, pressure: KickConfig) -> std::io::Result<()> {
    let mut file = writer(output, "generation-summary.tsv")?;
    writeln!(file, "field\tvalue")?;
    writeln!(file, "status\tengineering_pass_listening_open")?;
    writeln!(file, "wav_count\t4")?;
    writeln!(file, "sample_rate\t{SAMPLE_RATE}")?;
    writeln!(file, "house_modifier\t{:.6}", house.modifier_amount)?;
    writeln!(file, "pressure_coupling\t{:.6}", pressure.modifier_amount)?;
    writeln!(file, "forbidden_processors\tnone")
}

fn write_cost(output: &Path, seconds: f64) -> std::io::Result<()> {
    let mut file = writer(output, "workstation-cost.txt")?;
    writeln!(
        file,
        "Volatile workstation measurement; excluded from deterministic comparison."
    )?;
    writeln!(file, "wall_seconds={seconds:.6}")
}

fn writer(output: &Path, name: &str) -> std::io::Result<BufWriter<File>> {
    Ok(BufWriter::new(File::create(output.join(name))?))
}

fn stereo_sample_hash(samples: &[f32]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for &sample in samples {
        for _ in 0..2 {
            for byte in sample.to_bits().to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
    }
    hash
}

fn write_wav(path: &Path, render: &KickRender) -> Result<(), hound::Error> {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 2,
            sample_rate: render.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;
    for &sample in &render.samples {
        writer.write_sample(sample)?;
        writer.write_sample(sample)?;
    }
    writer.finalize()
}
