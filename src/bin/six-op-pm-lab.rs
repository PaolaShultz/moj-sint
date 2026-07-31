use anyhow::{Context, Result, bail, ensure};
use moj_sint::dsp::six_op_pm::algorithm::{CLASSIC_ALGORITHMS, PreparedAlgorithm};
use moj_sint::six_op_pm::{
    AliasRow, EvidenceStatus, GateEvidence, ListeningPatch, ListeningRole, Render, SpectralRow,
    SpectralStage, measure_listening_gate, render,
};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

const SAMPLE_RATE: u32 = 48_000;

fn main() -> ExitCode {
    match parse_command().and_then(|output| render_gate(&output)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn parse_command() -> Result<PathBuf> {
    let mut arguments = std::env::args_os().skip(1);
    let command = arguments.next();
    let output = arguments.next();
    if command.as_deref() != Some(std::ffi::OsStr::new("render"))
        || output.is_none()
        || arguments.next().is_some()
    {
        bail!("Usage: six-op-pm-lab render <output-directory>");
    }
    Ok(PathBuf::from(output.expect("output presence was checked")))
}

fn render_gate(output_directory: &Path) -> Result<()> {
    ensure_destination_is_empty(output_directory)?;

    let started = Instant::now();
    let renders = ListeningPatch::ALL
        .into_iter()
        .map(|patch| render(patch, SAMPLE_RATE))
        .collect::<Result<Vec<_>, _>>()?;
    let evidence = measure_listening_gate()?;
    verify_in_memory(&renders, &evidence)?;
    let reports = Reports::new(&renders, &evidence)?;
    let elapsed = started.elapsed();

    fs::create_dir_all(output_directory)
        .with_context(|| format!("create {}", output_directory.display()))?;
    for (patch, rendered) in ListeningPatch::ALL.into_iter().zip(&renders) {
        write_wav(
            &output_directory.join(patch.filename()),
            &rendered.samples,
            rendered.format.sample_rate,
        )?;
    }
    reports.write(output_directory)?;
    write_workstation_cost(output_directory, elapsed)?;
    println!(
        "wrote six-operator PM listening gate to {}",
        output_directory.display()
    );
    Ok(())
}

fn ensure_destination_is_empty(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    ensure!(path.is_dir(), "destination exists and is not a directory");
    ensure!(
        fs::read_dir(path)?.next().is_none(),
        "destination exists and is not empty"
    );
    Ok(())
}

fn verify_in_memory(renders: &[Render], evidence: &GateEvidence) -> Result<()> {
    ensure!(
        renders.len() == ListeningPatch::ALL.len(),
        "render inventory is incomplete"
    );
    ensure!(
        evidence.status == EvidenceStatus::Pass,
        "measurement gate failed: {:?}",
        evidence.errors
    );
    ensure!(
        evidence.errors.is_empty(),
        "measurement gate retained errors"
    );

    for ((expected, rendered), measured) in ListeningPatch::ALL
        .into_iter()
        .zip(renders)
        .zip(&evidence.patches)
    {
        ensure!(
            measured.patch == expected,
            "measurement inventory order changed"
        );
        ensure!(
            measured.status == EvidenceStatus::Pass,
            "{expected:?} did not pass"
        );
        ensure!(
            rendered.format.sample_rate == SAMPLE_RATE,
            "wrong sample rate"
        );
        ensure!(rendered.format.channels == 2, "wrong channel count");
        ensure!(rendered.format.bits_per_sample == 32, "wrong sample width");
        ensure!(rendered.format.float, "render is not floating point");
        ensure!(rendered.samples.len() % 2 == 0, "incomplete stereo frame");
        ensure!(
            rendered.samples.iter().all(|sample| sample.is_finite()),
            "non-finite render"
        );
        ensure!(
            rendered.diagnostics.non_finite_voices == 0,
            "recovered non-finite voice"
        );
        ensure!(
            rendered.diagnostics.clamp_contacts == 0,
            "render touched output ceiling"
        );
        ensure!(
            rendered.sample_hash == measured.metrics.sample_hash,
            "musical render hash disagrees with measurement"
        );
    }
    Ok(())
}

struct Reports {
    readme: String,
    manifest: String,
    metrics: String,
    pitch: String,
    spectral: String,
    alias_error: String,
    graphs: String,
    hashes: String,
}

impl Reports {
    fn new(renders: &[Render], evidence: &GateEvidence) -> Result<Self> {
        Ok(Self {
            readme: readme(),
            manifest: manifest(evidence),
            metrics: metrics(evidence),
            pitch: pitch(evidence),
            spectral: spectral(evidence),
            alias_error: alias_error(evidence),
            graphs: graphs()?,
            hashes: hashes(renders),
        })
    }

    fn write(&self, output: &Path) -> Result<()> {
        for (filename, contents) in [
            ("README.md", &self.readme),
            ("manifest.tsv", &self.manifest),
            ("metrics.tsv", &self.metrics),
            ("pitch.tsv", &self.pitch),
            ("spectral.tsv", &self.spectral),
            ("alias-error.tsv", &self.alias_error),
            ("graphs.tsv", &self.graphs),
            ("hashes.tsv", &self.hashes),
        ] {
            fs::write(output.join(filename), contents)?;
        }
        Ok(())
    }
}

fn readme() -> String {
    String::from(
        "# Moj Sint six-operator PM listening gate\n\
\n\
Start at a low playback level, then raise it only as needed. The float files are deliberately dry and are not acoustic-level-calibrated; this makes no acoustic SPL claim.\n\
\n\
Listen in pair order:\n\
\n\
1. `01_bell-metal_reference.wav`, then `02_fractured-metal_original.wav`\n\
2. `03_electric-piano-mallet_reference.wav`, then `04_glass-wood_original.wav`\n\
3. `05_brass-bass_reference.wav`, then `06_mechanical-stab_original.wav`\n\
\n\
Automated reports establish deterministic engineering boundaries; human listening is the authority on category recognition, partner difference, and musical usefulness. These are three topologies, not six synthesis families: each pair shares one graph and score while comparing independently authored parameters.\n\
\n\
No Yamaha patch, factory voice, SysEx data, firmware, source code, prose, artwork, branding, or third-party emulator implementation was copied. The connectivity catalog records functional routing facts with source-number provenance; all implementation, patches, scores, tables, prose, and reports were independently authored. This research has no compatibility claim and does not imply Yamaha affiliation.\n\
\n\
The files use one fixed gain per pair with no per-file normalization, no effects, no compressor, limiter, clipping, post-filter, or mastering. They are stereo 48 kHz 32-bit float WAVs carrying the same dry signal in both channels. This is an isolated offline experiment, not production integration, a preset proposal, live callback evidence, Raspberry Pi performance evidence, or a production polyphony claim.\n",
    )
}

fn manifest(evidence: &GateEvidence) -> String {
    let mut output =
        String::from("number\tpatch\tpair\trole\talgorithm\tscore\tfilename\tpair_gain\tstatus\n");
    for (index, row) in evidence.patches.iter().enumerate() {
        let patch = row.patch;
        writeln!(
            output,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{}",
            index + 1,
            patch_slug(patch),
            patch.pair() + 1,
            role_slug(patch.role()),
            patch.patch().algorithm,
            score_slug(patch),
            patch.filename(),
            patch.pair_gain(),
            status_slug(row.status),
        )
        .expect("writing to String cannot fail");
    }
    output
}

fn metrics(evidence: &GateEvidence) -> String {
    let mut output = String::from(
        "patch\tpeak\tactive_rms\tactive_rms_dbfs\tcrest_factor\tdc\tmaximum_jump\tceiling_contacts\tfinite\tstatus\n",
    );
    for row in &evidence.patches {
        let metrics = row.metrics;
        writeln!(
            output,
            "{}\t{:.9}\t{:.9}\t{:.6}\t{:.6}\t{:.9}\t{:.9}\t{}\t{}\t{}",
            patch_slug(row.patch),
            metrics.peak,
            metrics.active_rms,
            metrics.active_rms_dbfs,
            metrics.crest_factor,
            metrics.dc,
            metrics.maximum_jump,
            metrics.ceiling_contacts,
            metrics.finite,
            status_slug(row.status),
        )
        .expect("writing to String cannot fail");
    }
    output
}

fn pitch(evidence: &GateEvidence) -> String {
    let mut output = String::from(
        "patch\tnote\tfrequency_hz\tpitch_anchor_db\tpitch_error_cents\tpitch_strength_db\tinharmonic_rule\tstatus\n",
    );
    for patch in &evidence.patches {
        for row in &patch.pitch_rows {
            write!(
                output,
                "{}\t{}\t{:.6}\t{:.6}\t",
                patch_slug(row.patch),
                row.note,
                row.measured_frequency_hz.unwrap_or(row.anchor_frequency_hz),
                row.anchor_db,
            )
            .expect("writing to String cannot fail");
            write_optional(&mut output, row.pitch_error_cents, 6);
            output.push('\t');
            if !row.inharmonic_rule {
                write_optional(&mut output, row.pitch_strength_db, 6);
            }
            writeln!(
                output,
                "\t{}\t{}",
                row.inharmonic_rule,
                status_slug(row.status)
            )
            .expect("writing to String cannot fail");
        }
    }
    output
}

fn spectral(evidence: &GateEvidence) -> String {
    let mut output =
        String::from("patch\tnote\tstage\tharmonic_energy_db\tinharmonic_residual_db\n");
    for patch in &evidence.patches {
        for row in &patch.spectral_rows {
            write_spectral_row(&mut output, row);
        }
    }
    output
}

fn write_spectral_row(output: &mut String, row: &SpectralRow) {
    writeln!(
        output,
        "{}\t{}\t{}\t{:.6}\t{:.6}",
        patch_slug(row.patch),
        row.note,
        stage_slug(row.stage),
        row.harmonic_energy_db,
        row.inharmonic_residual_db,
    )
    .expect("writing to String cannot fail");
}

fn alias_error(evidence: &GateEvidence) -> String {
    let mut output = String::from("patch\tnote\talias_error_db\tfloor_db\tstatus\n");
    for patch in &evidence.patches {
        for row in &patch.alias_rows {
            write_alias_row(&mut output, row);
        }
    }
    output
}

fn write_alias_row(output: &mut String, row: &AliasRow) {
    writeln!(
        output,
        "{}\t{}\t{:.6}\t{:.6}\t{}",
        patch_slug(row.patch),
        row.note,
        row.alias_error_db,
        row.floor_db,
        status_slug(row.status),
    )
    .expect("writing to String cannot fail");
}

fn graphs() -> Result<String> {
    let mut output =
        String::from("graph\tsource_number\tedges\tcarriers\tfeedback_edge\tevaluation_order\n");
    for (graph, specification) in CLASSIC_ALGORITHMS.into_iter().enumerate() {
        let prepared = PreparedAlgorithm::new(specification)?;
        let edges = specification
            .edges
            .iter()
            .map(|edge| format!("{}>{}", edge.source + 1, edge.target + 1))
            .collect::<Vec<_>>()
            .join(",");
        let carriers = (1..=6)
            .filter(|operator| specification.carriers & (1 << (operator - 1)) != 0)
            .map(|operator| operator.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let evaluation_order = prepared
            .evaluation_order()
            .iter()
            .map(|operator| (operator + 1).to_string())
            .collect::<Vec<_>>()
            .join(",");
        writeln!(
            output,
            "{graph}\t{}\t{edges}\t{carriers}\t{}>{}\t{evaluation_order}",
            specification.id,
            specification.feedback.source + 1,
            specification.feedback.target + 1,
        )
        .expect("writing to String cannot fail");
    }
    Ok(output)
}

fn hashes(renders: &[Render]) -> String {
    let mut output = String::from("file\tfnv1a_interleaved_stereo_sample_hash\n");
    for (patch, rendered) in ListeningPatch::ALL.into_iter().zip(renders) {
        writeln!(
            output,
            "{}\t{:016x}",
            patch.filename(),
            rendered.sample_hash
        )
        .expect("writing to String cannot fail");
    }
    output
}

fn write_optional(output: &mut String, value: Option<f64>, decimals: usize) {
    if let Some(value) = value {
        write!(output, "{value:.decimals$}").expect("writing to String cannot fail");
    }
}

fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<()> {
    let specification = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, specification)?;
    for &sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}

fn write_workstation_cost(output: &Path, elapsed: Duration) -> Result<()> {
    let contents = format!(
        "scope=isolated_scalar_offline_render_and_measurement\nverification_elapsed_seconds={:.6}\nlimitation=volatile workstation development evidence; not callback timing, Raspberry Pi evidence, production latency, polyphony, or sound-quality evidence\n",
        elapsed.as_secs_f64()
    );
    fs::write(output.join("workstation-cost.txt"), contents)?;
    Ok(())
}

const fn patch_slug(patch: ListeningPatch) -> &'static str {
    match patch {
        ListeningPatch::BellMetal => "bell-metal",
        ListeningPatch::FracturedMetal => "fractured-metal",
        ListeningPatch::ElectricPianoMallet => "electric-piano-mallet",
        ListeningPatch::GlassWood => "glass-wood",
        ListeningPatch::BrassBass => "brass-bass",
        ListeningPatch::MechanicalStab => "mechanical-stab",
    }
}

const fn role_slug(role: ListeningRole) -> &'static str {
    match role {
        ListeningRole::Reference => "reference",
        ListeningRole::Original => "original",
    }
}

const fn score_slug(patch: ListeningPatch) -> &'static str {
    match patch {
        ListeningPatch::BellMetal | ListeningPatch::FracturedMetal => "bell-strikes",
        ListeningPatch::ElectricPianoMallet | ListeningPatch::GlassWood => {
            "mallet-single-and-chord"
        }
        ListeningPatch::BrassBass | ListeningPatch::MechanicalStab => "brass-low-mid-phrase",
    }
}

const fn status_slug(status: EvidenceStatus) -> &'static str {
    match status {
        EvidenceStatus::Pass => "pass",
        EvidenceStatus::Fail => "fail",
    }
}

const fn stage_slug(stage: SpectralStage) -> &'static str {
    match stage {
        SpectralStage::Attack => "attack",
        SpectralStage::Sustain => "sustain",
        SpectralStage::Release => "release",
    }
}
