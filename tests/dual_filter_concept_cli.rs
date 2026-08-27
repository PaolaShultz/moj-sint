use std::collections::HashSet;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::process::Command;

const EXPECTED_FILES: [&str; 20] = [
    "01 LEAD ROTARY 01 - A Cutoff.wav",
    "02 LEAD ROTARY 02 - A Resonance.wav",
    "03 LEAD ROTARY 03 - A Envelope Depth.wav",
    "04 LEAD ROTARY 04 - B Cutoff.wav",
    "05 LEAD ROTARY 05 - B Resonance.wav",
    "06 LEAD ROTARY 06 - B Envelope Depth.wav",
    "07 LEAD ROTARY 07 - Routing.wav",
    "08 LEAD ROTARY 08 - Filter Attack.wav",
    "09 LEAD ROTARY 09 - Filter Decay.wav",
    "10 LEAD ROTARY 10 - Filter Sustain.wav",
    "11 LEAD ROTARY 11 - Filter Release.wav",
    "12 LEAD ROTARY 12 - Amp Attack.wav",
    "13 LEAD ROTARY 13 - Amp Decay.wav",
    "14 LEAD ROTARY 14 - Amp Sustain.wav",
    "15 LEAD ROTARY 15 - Amp Release.wav",
    "16 LEAD PUSH - Serial Parallel Topology.wav",
    "17 BASS 01 - Serial Cutoff and Resonance.wav",
    "18 BASS 02 - Countermotion Growl and Routing.wav",
    "19 BASS 03 - Envelope Punch.wav",
    "20 BASS 04 - Topology Push.wav",
];

#[test]
fn lab_writes_only_deterministic_bounded_moving_control_wavs() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for destination in [first.path().join("lab"), second.path().join("lab")] {
        let output = Command::new(env!("CARGO_BIN_EXE_dual-filter-envelope-lab"))
            .arg(&destination)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let mut names: Vec<_> = fs::read_dir(&destination)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, EXPECTED_FILES);
    }

    let mut hashes = HashSet::new();
    for file in EXPECTED_FILES {
        let first_bytes = fs::read(first.path().join("lab").join(file)).unwrap();
        let second_bytes = fs::read(second.path().join("lab").join(file)).unwrap();
        assert_eq!(first_bytes, second_bytes, "file={file}");

        let mut hasher = DefaultHasher::new();
        first_bytes.hash(&mut hasher);
        assert!(
            hashes.insert(hasher.finish()),
            "duplicate audio file={file}"
        );

        let mut reader = hound::WavReader::open(first.path().join("lab").join(file)).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 2, "file={file}");
        assert_eq!(spec.sample_rate, 48_000, "file={file}");
        assert_eq!(spec.bits_per_sample, 32, "file={file}");
        assert_eq!(
            spec.sample_format,
            hound::SampleFormat::Float,
            "file={file}"
        );
        let mut peak = 0.0_f32;
        for sample in reader.samples::<f32>() {
            let sample = sample.unwrap();
            assert!(sample.is_finite(), "file={file}");
            peak = peak.max(sample.abs());
        }
        assert!(peak > 1.0e-4 && peak <= 1.0, "file={file} peak={peak}");
    }
}
