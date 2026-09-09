#![cfg(feature = "open303")]
use std::process::Command;
#[test]
fn offline_comparison_is_deterministic_and_never_overwrites() {
    let temp = tempfile::tempdir().unwrap();
    for dir in ["first", "second"] {
        let output = Command::new(env!("CARGO_BIN_EXE_open303-lab"))
            .arg(temp.path().join(dir))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for file in [
        "01-coupled-303.wav",
        "02-cascade-18.wav",
        "measurements.txt",
    ] {
        assert_eq!(
            std::fs::read(temp.path().join("first").join(file)).unwrap(),
            std::fs::read(temp.path().join("second").join(file)).unwrap()
        );
    }
    let a = std::fs::read(temp.path().join("first/01-coupled-303.wav")).unwrap();
    let b = std::fs::read(temp.path().join("first/02-cascade-18.wav")).unwrap();
    assert_ne!(a, b);
    let output = Command::new(env!("CARGO_BIN_EXE_open303-lab"))
        .arg(temp.path().join("first"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        a,
        std::fs::read(temp.path().join("first/01-coupled-303.wav")).unwrap()
    );
    let mut reader = hound::WavReader::new(std::io::Cursor::new(a)).unwrap();
    assert_eq!(reader.spec().channels, 2);
    assert_eq!(reader.spec().sample_rate, 48000);
    let data: Vec<f32> = reader.samples().map(Result::unwrap).collect();
    assert!(data.iter().all(|x| x.is_finite() && x.abs() <= 0.999));
    assert!(data.iter().any(|x| x.abs() > 0.001));
}
