use moj_sint::offline::{RenderSpec, write_wav};
use moj_sint::preset::Preset;
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "Usage:\n  moj-sint validate <preset.mojsint>\n  moj-sint render <preset.mojsint> <output.wav> [--note 0..127] [--seconds N] [--sample-rate HZ] [--velocity 0..1]";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}\n\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("--help" | "-h") => {
            println!("{USAGE}");
            Ok(())
        }
        Some("validate") if args.len() == 2 => {
            let preset = read_preset(&args[1])?;
            println!("valid Moj Sint preset: {}", preset.name);
            Ok(())
        }
        Some("render") if args.len() >= 3 => render(&args[1..]),
        Some(command) => Err(format!("unknown or incomplete command `{command}`")),
        None => Err("missing command".into()),
    }
}

fn read_preset(path: impl AsRef<Path>) -> Result<Preset, String> {
    let path = path.as_ref();
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Preset::parse(&source).map_err(|error| format!("invalid preset {}: {error}", path.display()))
}

fn render(args: &[String]) -> Result<(), String> {
    let preset = read_preset(&args[0])?;
    let mut spec = RenderSpec {
        sample_rate: 48_000,
        note: 60,
        velocity: 0.8,
        seconds: 1.0,
    };
    let mut index = 2;
    while index < args.len() {
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {}", args[index]))?;
        match args[index].as_str() {
            "--note" => {
                let note: u16 = value
                    .parse()
                    .map_err(|_| "MIDI note must be an integer between 0 and 127".to_string())?;
                spec.note = u8::try_from(note)
                    .ok()
                    .filter(|note| *note <= 127)
                    .ok_or_else(|| "MIDI note must be between 0 and 127".to_string())?;
            }
            "--seconds" => {
                spec.seconds = value
                    .parse()
                    .map_err(|_| "seconds must be a number".to_string())?
            }
            "--sample-rate" => {
                spec.sample_rate = value
                    .parse()
                    .map_err(|_| "sample rate must be an integer".to_string())?
            }
            "--velocity" => {
                spec.velocity = value
                    .parse()
                    .map_err(|_| "velocity must be a number".to_string())?
            }
            flag => return Err(format!("unknown option `{flag}`")),
        }
        index += 2;
    }
    write_wav(&args[1], &preset, spec).map_err(|error| error.to_string())?;
    println!("rendered {}", args[1]);
    Ok(())
}
