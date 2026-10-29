use std::{error::Error, fs::File, io::BufWriter, path::PathBuf};

use truespeech::{TRUESPEECH_SAMPLE_RATE, write_truespeech_wav};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let destination = match arguments.next() {
        None => PathBuf::from("speech.wav"),
        Some(argument) if matches!(argument.to_str(), Some("-h" | "-help" | "--help")) => {
            if arguments.next().is_some() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "help options cannot be combined with other arguments",
                )
                .into());
            }
            println!(
                "Usage: encode_wav [OUTPUT.wav]\n       encode_wav -h | -help | --help\n\nCreates a two-second 440 Hz TrueSpeech WAV file. The output path defaults to speech.wav."
            );
            return Ok(());
        }
        Some(argument) => {
            if arguments.next().is_some() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Usage: encode_wav [OUTPUT.wav]",
                )
                .into());
            }
            PathBuf::from(argument)
        }
    };

    // A quiet 440 Hz tone demonstrates the complete PCM-to-WAV path. Replace
    // this vector with mono signed 16-bit PCM sampled at 8 kHz in applications.
    let sample_count = TRUESPEECH_SAMPLE_RATE as usize * 2;
    let samples: Vec<i16> = (0..sample_count)
        .map(|index| {
            let phase =
                std::f64::consts::TAU * 440.0 * index as f64 / f64::from(TRUESPEECH_SAMPLE_RATE);
            (phase.sin() * 8_000.0) as i16
        })
        .collect();

    let output = BufWriter::new(File::create(&destination)?);
    write_truespeech_wav(output, &samples)?;
    println!(
        "Wrote {} samples to {}",
        samples.len(),
        destination.display()
    );
    Ok(())
}
