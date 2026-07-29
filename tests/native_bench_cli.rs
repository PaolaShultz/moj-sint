use std::process::Command;

#[test]
fn smoke_run_writes_raw_summary_and_platform_evidence() {
    let temporary = tempfile::tempdir().unwrap();
    let output = temporary.path().join("native-smoke");
    let status = Command::new(std::env::var("CARGO_BIN_EXE_native-pi-bench").unwrap())
        .args(["smoke", output.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());

    for relative in [
        "platform.txt",
        "prescan-engine.tsv",
        "summary.tsv",
        "safety.tsv",
        "raw/production-engine_held-note_v1_f64_t1.tsv",
    ] {
        let path = output.join(relative);
        assert!(path.is_file(), "missing {}", path.display());
    }
    let summary = std::fs::read_to_string(output.join("summary.tsv")).unwrap();
    assert!(summary.contains("p999_ns"));
    assert!(summary.contains("production-engine"));
}
