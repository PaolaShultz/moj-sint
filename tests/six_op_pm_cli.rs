use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Output, Stdio};

const WAV_NAMES: [&str; 6] = [
    "01_bell-metal_reference.wav",
    "02_fractured-metal_original.wav",
    "03_electric-piano-mallet_reference.wav",
    "04_glass-wood_original.wav",
    "05_brass-bass_reference.wav",
    "06_mechanical-stab_original.wav",
];

const DETERMINISTIC_REPORTS: [&str; 8] = [
    "README.md",
    "alias-error.tsv",
    "graphs.tsv",
    "hashes.tsv",
    "manifest.tsv",
    "metrics.tsv",
    "pitch.tsv",
    "spectral.tsv",
];

#[test]
#[ignore = "development-only six-operator listening-gate renderer"]
fn six_operator_lab_writes_the_exact_deterministic_listening_gate() {
    let first = tempfile::tempdir().unwrap();
    let second_parent = tempfile::tempdir().unwrap();
    let second = second_parent
        .path()
        .join("missing")
        .join("nested")
        .join("new-listening-gate");
    run_lab(first.path()).assert_success();
    assert!(!second_parent.path().join("missing").exists());
    assert!(!second.exists());
    run_lab(&second).assert_success();

    let first_files = deterministic_files(first.path());
    let second_files = deterministic_files(&second);
    assert_eq!(first_files, second_files);

    let expected: BTreeSet<_> = WAV_NAMES
        .into_iter()
        .chain(DETERMINISTIC_REPORTS)
        .chain(["workstation-cost.txt"])
        .map(str::to_owned)
        .collect();
    assert_eq!(file_names(first.path()), expected);

    let hashes = hash_rows(first.path());
    assert_eq!(hashes.len(), WAV_NAMES.len());
    for filename in WAV_NAMES {
        let mut reader = hound::WavReader::open(first.path().join(filename)).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 2, "{filename}");
        assert_eq!(spec.sample_rate, 48_000, "{filename}");
        assert_eq!(spec.bits_per_sample, 32, "{filename}");
        assert_eq!(spec.sample_format, hound::SampleFormat::Float, "{filename}");
        assert_eq!(reader.len() % 2, 0, "{filename}");
        let samples: Vec<f32> = reader.samples::<f32>().collect::<Result<_, _>>().unwrap();
        assert!(!samples.is_empty(), "{filename}");
        assert!(
            samples.iter().all(|sample| sample.is_finite()),
            "{filename}"
        );
        for frame in samples.chunks_exact(2) {
            assert_eq!(frame[0].to_bits(), frame[1].to_bits(), "{filename}");
        }
        assert_eq!(sample_hash(&samples), hashes[filename], "{filename}");
    }

    assert_header(
        first.path(),
        "manifest.tsv",
        "number\tpatch\tpair\trole\talgorithm\tscore\tfilename\tpair_gain\tstatus",
    );
    assert_header(
        first.path(),
        "metrics.tsv",
        "patch\tpeak\tactive_rms\tactive_rms_dbfs\tcrest_factor\tdc\tmaximum_jump\tceiling_contacts\tfinite\tstatus",
    );
    assert_header(
        first.path(),
        "pitch.tsv",
        "patch\tnote\tfrequency_hz\tpitch_anchor_db\tpitch_error_cents\tpitch_strength_db\tinharmonic_rule\tstatus",
    );
    assert_header(
        first.path(),
        "spectral.tsv",
        "patch\tnote\tstage\tharmonic_energy_db\tinharmonic_residual_db",
    );
    assert_header(
        first.path(),
        "alias-error.tsv",
        "patch\tnote\talias_error_db\tfloor_db\tstatus",
    );
    assert_header(
        first.path(),
        "graphs.tsv",
        "graph\tsource_number\tedges\tcarriers\tfeedback_edge\tevaluation_order",
    );
    assert_header(
        first.path(),
        "hashes.tsv",
        "file\tfnv1a_interleaved_stereo_sample_hash",
    );

    assert_eq!(line_count(first.path(), "manifest.tsv"), 7);
    assert_eq!(line_count(first.path(), "metrics.tsv"), 7);
    assert_eq!(line_count(first.path(), "pitch.tsv"), 19);
    assert_eq!(line_count(first.path(), "spectral.tsv"), 55);
    assert_eq!(line_count(first.path(), "alias-error.tsv"), 19);
    assert_eq!(line_count(first.path(), "graphs.tsv"), 33);
    assert_eq!(line_count(first.path(), "hashes.tsv"), 7);

    for (filename, status_column) in [
        ("manifest.tsv", 8),
        ("metrics.tsv", 9),
        ("pitch.tsv", 7),
        ("alias-error.tsv", 4),
    ] {
        let report = fs::read_to_string(first.path().join(filename)).unwrap();
        for row in report.lines().skip(1) {
            let fields: Vec<_> = row.split('\t').collect();
            assert_eq!(fields[status_column], "pass", "{filename}: {row}");
        }
    }

    let pitch = fs::read_to_string(first.path().join("pitch.tsv")).unwrap();
    for row in pitch.lines().skip(1) {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 8, "{row}");
        if fields[6] == "true" {
            assert!(
                fields[4].is_empty(),
                "inharmonic pitch error must be blank: {row}"
            );
            assert!(
                fields[5].is_empty(),
                "inharmonic pitch strength must be blank: {row}"
            );
        } else {
            assert!(!fields[4].is_empty(), "harmonic pitch error missing: {row}");
            assert!(
                !fields[5].is_empty(),
                "harmonic pitch strength missing: {row}"
            );
        }
        assert_eq!(fields[7], "pass", "{row}");
    }

    let readme = fs::read_to_string(first.path().join("README.md")).unwrap();
    for boundary in [
        "Start at a low playback level",
        "Listen in pair order",
        "human listening is the authority",
        "three topologies, not six synthesis families",
        "No Yamaha patch",
        "no per-file normalization",
        "no effects",
        "no compatibility claim",
        "no acoustic SPL claim",
        "not production integration",
    ] {
        assert!(readme.contains(boundary), "README missing: {boundary}");
    }
}

#[test]
fn six_operator_lab_accepts_exactly_render_and_one_destination() {
    for arguments in [
        vec![],
        vec!["render"],
        vec!["other", "output"],
        vec!["render", "output", "extra"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_six-op-pm-lab"))
            .args(arguments)
            .output()
            .unwrap();
        assert!(!result.status.success(), "{result:?}");
        assert!(
            String::from_utf8_lossy(&result.stderr)
                .contains("Usage: six-op-pm-lab render <output-directory>"),
            "{result:?}"
        );
    }
}

#[test]
fn six_operator_lab_rejects_unsafe_destinations_without_touching_cwd() {
    let current = tempfile::tempdir().unwrap();
    fs::write(current.path().join("caller-sentinel.txt"), b"caller bytes").unwrap();

    for (destination, expected_error) in [
        ("", "destination must have a safe final component"),
        (".", "destination must have a safe final component"),
        ("..", "destination must not contain parent traversal"),
        ("nested/.", "destination must have a safe final component"),
        ("nested/..", "destination must not contain parent traversal"),
        ("/", "destination must have a safe final component"),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_six-op-pm-lab"))
            .current_dir(current.path())
            .arg("render")
            .arg(OsStr::new(destination))
            .output()
            .unwrap();
        assert!(!result.status.success(), "{destination:?}: {result:?}");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(expected_error),
            "{destination:?}: {result:?}"
        );
    }
    let parent_traversal = current.path().join("detour").join("..").join("gate");
    let result = Command::new(env!("CARGO_BIN_EXE_six-op-pm-lab"))
        .arg("render")
        .arg(&parent_traversal)
        .output()
        .unwrap();
    assert!(!result.status.success(), "{result:?}");
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("must not contain parent traversal"),
        "{result:?}"
    );
    assert!(!current.path().join("detour").exists());
    assert!(!current.path().join("gate").exists());
    assert_eq!(
        file_names(current.path()),
        BTreeSet::from(["caller-sentinel.txt".to_owned()])
    );
    assert_eq!(
        fs::read(current.path().join("caller-sentinel.txt")).unwrap(),
        b"caller bytes"
    );
}

#[test]
fn six_operator_lab_refuses_a_nonempty_destination_without_touching_it() {
    let output = tempfile::tempdir().unwrap();
    fs::write(output.path().join("keep.txt"), b"owned by caller").unwrap();

    let result = run_lab(output.path());
    assert!(!result.status.success(), "{result:?}");
    assert_eq!(
        file_names(output.path()),
        BTreeSet::from(["keep.txt".to_owned()])
    );
    assert_eq!(
        fs::read(output.path().join("keep.txt")).unwrap(),
        b"owned by caller"
    );
}

#[test]
#[ignore = "development-only six-operator publication race renderer"]
fn six_operator_lab_loses_a_late_destination_race_without_overwrite_or_residue() {
    let parent = tempfile::tempdir().unwrap();
    let target = parent.path().join("listening-gate");
    let caller_wav = target.join(WAV_NAMES[0]);
    fs::write(parent.path().join("caller-sentinel.txt"), b"parent owned").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_six-op-pm-lab"))
        .arg("render")
        .arg(&target)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut child_stdout = BufReader::new(child.stdout.take().unwrap());
    let mut progress = String::new();
    child_stdout.read_line(&mut progress).unwrap();
    assert!(
        child.try_wait().unwrap().is_none(),
        "render ended before race setup"
    );
    assert_eq!(
        progress,
        "verifying six-operator PM listening gate before publication\n"
    );
    fs::create_dir(&target).unwrap();
    fs::write(&caller_wav, b"caller-owned predictable wav bytes").unwrap();

    let result = child.wait_with_output().unwrap();
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("destination exists and is not empty"),
        "{result:?}"
    );
    assert_eq!(
        fs::read(&caller_wav).unwrap(),
        b"caller-owned predictable wav bytes"
    );
    assert_eq!(
        file_names(&target),
        BTreeSet::from([WAV_NAMES[0].to_owned()])
    );
    assert_eq!(
        file_names(parent.path()),
        BTreeSet::from([
            "caller-sentinel.txt".to_owned(),
            "listening-gate".to_owned()
        ])
    );
}

fn run_lab(output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_six-op-pm-lab"))
        .arg("render")
        .arg(output)
        .output()
        .unwrap()
}

fn deterministic_files(directory: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_name() != "workstation-cost.txt")
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let bytes = fs::read(entry.path()).unwrap();
            (name, bytes)
        })
        .collect()
}

fn file_names(directory: &Path) -> BTreeSet<String> {
    fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

fn assert_header(directory: &Path, filename: &str, expected: &str) {
    let contents = fs::read_to_string(directory.join(filename)).unwrap();
    assert_eq!(contents.lines().next(), Some(expected), "{filename}");
}

fn line_count(directory: &Path, filename: &str) -> usize {
    fs::read_to_string(directory.join(filename))
        .unwrap()
        .lines()
        .count()
}

fn hash_rows(directory: &Path) -> BTreeMap<String, u64> {
    fs::read_to_string(directory.join("hashes.tsv"))
        .unwrap()
        .lines()
        .skip(1)
        .map(|row| {
            let (filename, hash) = row.split_once('\t').unwrap();
            (filename.to_owned(), u64::from_str_radix(hash, 16).unwrap())
        })
        .collect()
}

fn sample_hash(samples: &[f32]) -> u64 {
    samples.iter().fold(0xcbf2_9ce4_8422_2325, |hash, sample| {
        (hash ^ u64::from(sample.to_bits())).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

trait OutputAssertion {
    fn assert_success(&self);
}

impl OutputAssertion for Output {
    fn assert_success(&self) {
        assert!(
            self.status.success(),
            "status={}\nstdout={}\nstderr={}",
            self.status,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        );
    }
}
