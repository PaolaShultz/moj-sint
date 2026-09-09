//! Disposable offline comparison of Open303's two distinct filter structures.
use moj_sint::open303::{Controls, FilterMode, Open303};
use std::{fmt::Write as _, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let directory = PathBuf::from(args.next().ok_or("usage: open303-lab OUTPUT_DIRECTORY")?);
    if args.next().is_some() {
        return Err("usage: open303-lab OUTPUT_DIRECTORY".into());
    }
    // Atomic refusal of an existing directory; never overwrite a listening batch.
    fs::create_dir(&directory)?;
    let mut report = String::from(
        "Open303 isolated candidate\n48000 Hz dual mono; identical notes, accents, slides and gain\nNo playback or hardware measurement. This is not a factory preset bank.\nThe +/-0.999 presentation ceiling is outside the inherited DSP.\n\n",
    );
    for (mode, filename) in [
        (FilterMode::Tb303, "01-coupled-303.wav"),
        (FilterMode::Lowpass18, "02-cascade-18.wav"),
    ] {
        let mut voice = Open303::new(
            48000.0,
            mode,
            Controls {
                waveform: 0.65,
                cutoff_hz: 900.0,
                resonance: 75.0,
                env_mod: 70.0,
                accent: 60.0,
                ..Controls::default()
            },
        )?;
        let mut samples = vec![0.0; 6 * 48000];
        let notes = [
            36, 36, 43, 39, 36, 48, 43, 39, 34, 34, 41, 37, 34, 46, 41, 37,
        ];
        let accent = [
            true, false, false, true, false, true, false, false, true, false, false, true, false,
            true, false, false,
        ];
        let slide = [
            false, true, false, false, true, false, false, false, false, true, false, false, true,
            false, false, false,
        ];
        for step in 0..notes.len() {
            voice.note_on(notes[step], if accent[step] { 110 } else { 75 })?;
            if step > 0 && notes[step - 1] != notes[step] {
                voice.note_off(notes[step - 1]);
            }
            let start = step * 12000;
            voice.render(&mut samples[start..start + 9000]);
            if !slide[step] {
                voice.note_off(notes[step]);
            }
            voice.render(&mut samples[start + 9000..start + 12000]);
        }
        voice.release_all();
        voice.render(&mut samples[16 * 12000..]);
        let status = voice.status();
        if status.faulted {
            return Err("non-finite internal output: candidate batch rejected".into());
        }
        let peak = samples.iter().map(|x| x.abs()).fold(0.0_f32, f32::max);
        let rms = (samples.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>()
            / samples.len() as f64)
            .sqrt();
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut wav = hound::WavWriter::create(directory.join(filename), spec)?;
        for x in &samples {
            wav.write_sample(*x)?;
            wav.write_sample(*x)?;
        }
        wav.finalize()?;
        writeln!(
            report,
            "{filename}: peak={peak:.9}, rms={rms:.9}, clipped_samples={}, idle={}",
            status.clipped_samples,
            voice.is_idle()
        )?;
    }
    fs::write(directory.join("measurements.txt"), report)?;
    println!(
        "Wrote two filter-topology comparisons to {}",
        directory.display()
    );
    Ok(())
}
