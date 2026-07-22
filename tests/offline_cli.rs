use std::process::Command;

#[test]
fn validate_accepts_reference_preset() {
    let output = Command::new(env!("CARGO_BIN_EXE_moj-sint"))
        .args(["validate", "presets/reference.mojsint"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("valid"));
}

#[test]
fn render_writes_a_valid_wav() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("note.wav");
    let output = Command::new(env!("CARGO_BIN_EXE_moj-sint"))
        .args(["render", "presets/reference.mojsint"])
        .arg(&output_path)
        .args(["--note", "60", "--seconds", "0.05"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(hound::WavReader::open(output_path).is_ok());
}

#[test]
fn invalid_arguments_fail_clearly() {
    let output = Command::new(env!("CARGO_BIN_EXE_moj-sint"))
        .args([
            "render",
            "presets/reference.mojsint",
            "/tmp/unused.wav",
            "--note",
            "300",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("MIDI note"));
}
