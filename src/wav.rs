//! TrueSpeech-compressed WAV file writing utilities.

use std::io::{self, Write};

use crate::{TRUESPEECH_FRAME_SAMPLES, TRUESPEECH_PACKET_BYTES, TrueSpeechContext};

const TRUE_SPEECH_FORMAT_TAG: u16 = 0x0022;
const TRUE_SPEECH_AVERAGE_BYTES_PER_SECOND: u32 = 1_067;
const TRUE_SPEECH_FORMAT_CHUNK_SIZE: u32 = 32;
const TRUE_SPEECH_RIFF_OVERHEAD: usize = 64;
const WAV_HEADER_SIZE: usize = 72;

/// Writes a complete WAV file and zero-pads the final frame when needed.
///
/// The file contains the extended TrueSpeech `fmt ` chunk and a `fact` chunk
/// that records the input duration plus the encoder's 60-sample delay. This
/// function writes to `Write` directly and does not buffer all encoded packets.
///
/// # Input
/// `pcm_samples` must contain mono signed 16-bit PCM sampled at 8 kHz.
///
/// # Output
/// Returns `Ok(())` after all WAV bytes are written. Returns `TrueSpeechWavError`
/// if the RIFF size limit is exceeded or the destination writer fails.
pub fn write_truespeech_wav<W: Write>(
    mut output: W,
    pcm_samples: &[i16],
) -> Result<(), TrueSpeechWavError> {
    let (sample_count, packet_data_size, riff_size) = wav_sizes(pcm_samples.len())?;

    output.write_all(b"RIFF")?;
    output.write_all(&riff_size.to_le_bytes())?;
    output.write_all(b"WAVE")?;

    output.write_all(b"fmt ")?;
    output.write_all(&TRUE_SPEECH_FORMAT_CHUNK_SIZE.to_le_bytes())?;
    output.write_all(&TRUE_SPEECH_FORMAT_TAG.to_le_bytes())?;
    output.write_all(&1u16.to_le_bytes())?;
    output.write_all(&8_000u32.to_le_bytes())?;
    output.write_all(&TRUE_SPEECH_AVERAGE_BYTES_PER_SECOND.to_le_bytes())?;
    output.write_all(&(TRUESPEECH_PACKET_BYTES as u16).to_le_bytes())?;
    output.write_all(&1u16.to_le_bytes())?;
    output.write_all(&14u16.to_le_bytes())?;
    output.write_all(&1u16.to_le_bytes())?;
    output.write_all(&(TRUESPEECH_FRAME_SAMPLES as u16).to_le_bytes())?;
    output.write_all(&[0u8; 10])?;

    output.write_all(b"fact")?;
    output.write_all(&4u32.to_le_bytes())?;
    output.write_all(&sample_count.to_le_bytes())?;

    output.write_all(b"data")?;
    output.write_all(&packet_data_size.to_le_bytes())?;

    let mut encoder = TrueSpeechContext::new();
    let mut packet = [0u8; TRUESPEECH_PACKET_BYTES];
    let (complete_frames, partial_frame) = pcm_samples.as_chunks::<TRUESPEECH_FRAME_SAMPLES>();
    for input_frame in complete_frames {
        encoder.encode_frame(input_frame, &mut packet);
        output.write_all(&packet)?;
    }
    if !partial_frame.is_empty() {
        let mut padded_frame = [0i16; TRUESPEECH_FRAME_SAMPLES];
        padded_frame[..partial_frame.len()].copy_from_slice(partial_frame);
        encoder.encode_frame(&padded_frame, &mut packet);
        output.write_all(&packet)?;
    }
    if !pcm_samples.is_empty()
        && (partial_frame.is_empty() || partial_frame.len() > TRUESPEECH_FRAME_SAMPLES - 60)
    {
        encoder.encode_frame(&[0i16; TRUESPEECH_FRAME_SAMPLES], &mut packet);
        output.write_all(&packet)?;
    }

    Ok(())
}

/// Encodes PCM into a complete TrueSpeech WAV file held in memory.
///
/// # Input
/// `pcm_samples` must contain mono signed 16-bit PCM sampled at 8 kHz.
///
/// # Output
/// Returns the complete RIFF/WAV byte vector, or a `TrueSpeechWavError` if the
/// file exceeds the RIFF size limit or memory reservation fails. For streaming
/// output, use [`write_truespeech_wav`].
pub fn encode_truespeech_wav(pcm_samples: &[i16]) -> Result<Vec<u8>, TrueSpeechWavError> {
    let (_, packet_data_size, _) = wav_sizes(pcm_samples.len())?;
    let file_size = WAV_HEADER_SIZE
        .checked_add(packet_data_size as usize)
        .ok_or(TrueSpeechWavError::TooLarge)?;
    let mut wav_bytes = Vec::new();
    wav_bytes
        .try_reserve_exact(file_size)
        .map_err(|_| TrueSpeechWavError::AllocationFailed)?;
    write_truespeech_wav(&mut wav_bytes, pcm_samples)?;
    Ok(wav_bytes)
}

fn wav_sizes(sample_count: usize) -> Result<(u32, u32, u32), TrueSpeechWavError> {
    let fact_samples = if sample_count == 0 {
        0
    } else {
        sample_count
            .checked_add(60)
            .ok_or(TrueSpeechWavError::TooLarge)?
    };
    let fact_samples = u32::try_from(fact_samples).map_err(|_| TrueSpeechWavError::TooLarge)?;
    let mut frame_count = sample_count.div_ceil(TRUESPEECH_FRAME_SAMPLES);
    let remainder = sample_count % TRUESPEECH_FRAME_SAMPLES;
    if sample_count > 0 && (remainder == 0 || remainder > TRUESPEECH_FRAME_SAMPLES - 60) {
        frame_count = frame_count
            .checked_add(1)
            .ok_or(TrueSpeechWavError::TooLarge)?;
    }
    let packet_data_size = frame_count
        .checked_mul(TRUESPEECH_PACKET_BYTES)
        .ok_or(TrueSpeechWavError::TooLarge)?;
    let packet_data_size =
        u32::try_from(packet_data_size).map_err(|_| TrueSpeechWavError::TooLarge)?;
    let riff_size = TRUE_SPEECH_RIFF_OVERHEAD
        .checked_add(packet_data_size as usize)
        .and_then(|size| u32::try_from(size).ok())
        .ok_or(TrueSpeechWavError::TooLarge)?;
    Ok((fact_samples, packet_data_size, riff_size))
}

/// Errors returned while creating a TrueSpeech WAV file.
#[derive(Debug)]
pub enum TrueSpeechWavError {
    TooLarge,
    AllocationFailed,
    Io(io::Error),
}

impl std::fmt::Display for TrueSpeechWavError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => {
                formatter.write_str("the TrueSpeech WAV exceeds the RIFF 32-bit size limit")
            }
            Self::AllocationFailed => formatter.write_str("could not reserve memory for the WAV"),
            Self::Io(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for TrueSpeechWavError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::TooLarge | Self::AllocationFailed => None,
        }
    }
}

impl From<io::Error> for TrueSpeechWavError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_aligned_input_flushes_delayed_tail() {
        let mut samples = [0i16; TRUESPEECH_FRAME_SAMPLES];
        samples[180..].fill(16000);
        let wav = encode_truespeech_wav(&samples).unwrap();
        assert_eq!(u32::from_le_bytes(wav[60..64].try_into().unwrap()), 300);
        assert_eq!(u32::from_le_bytes(wav[68..72].try_into().unwrap()), 64);
        let mut context = TrueSpeechContext::new();
        let mut packet = [0u8; 32];
        context.encode_frame(&samples, &mut packet);
        context.encode_frame(&[0; 240], &mut packet);
        assert_eq!(&wav[104..136], &packet);

        let Ok(mut child) = std::process::Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-i",
                "pipe:0",
                "-f",
                "s16le",
                "-acodec",
                "pcm_s16le",
                "pipe:1",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        else {
            return;
        };
        use std::io::Write;
        child.stdin.take().unwrap().write_all(&wav).unwrap();
        let decoded = child.wait_with_output().unwrap();
        assert!(decoded.status.success());
        let tail: Vec<i16> = decoded.stdout[480..600]
            .chunks_exact(2)
            .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
            .collect();
        assert!(
            tail.iter()
                .map(|sample| i32::from(*sample).abs())
                .max()
                .unwrap_or(0)
                > 100
        );
    }

    #[test]
    fn wav_frame_count_covers_the_flush_delay_at_partial_frame_boundaries() {
        assert_eq!(wav_sizes(180).unwrap().1, TRUESPEECH_PACKET_BYTES as u32);
        assert_eq!(
            wav_sizes(181).unwrap().1,
            2 * TRUESPEECH_PACKET_BYTES as u32
        );
        assert_eq!(
            wav_sizes(239).unwrap().1,
            2 * TRUESPEECH_PACKET_BYTES as u32
        );
        assert_eq!(
            wav_sizes(240).unwrap().1,
            2 * TRUESPEECH_PACKET_BYTES as u32
        );
        assert_eq!(
            wav_sizes(241).unwrap().1,
            2 * TRUESPEECH_PACKET_BYTES as u32
        );
    }

    #[test]
    fn output_has_true_speech_format_fact_and_padded_packet_data() {
        let pcm_samples = [123i16; TRUESPEECH_FRAME_SAMPLES + 1];
        let wav_bytes = encode_truespeech_wav(&pcm_samples);
        assert!(wav_bytes.is_ok());
        let wav_bytes = match wav_bytes {
            Ok(bytes) => bytes,
            Err(error) => panic!("WAV encoding failed: {error}"),
        };

        assert_eq!(&wav_bytes[..4], b"RIFF");
        assert_eq!(
            u32::from_le_bytes([wav_bytes[4], wav_bytes[5], wav_bytes[6], wav_bytes[7]]) as usize,
            wav_bytes.len() - 8
        );
        assert_eq!(&wav_bytes[8..12], b"WAVE");
        assert_eq!(&wav_bytes[12..16], b"fmt ");
        assert_eq!(
            u16::from_le_bytes([wav_bytes[20], wav_bytes[21]]),
            TRUE_SPEECH_FORMAT_TAG
        );
        assert_eq!(&wav_bytes[52..56], b"fact");
        assert_eq!(
            u32::from_le_bytes([wav_bytes[60], wav_bytes[61], wav_bytes[62], wav_bytes[63]]),
            (pcm_samples.len() + 60) as u32
        );
        assert_eq!(&wav_bytes[64..68], b"data");
        assert_eq!(
            u32::from_le_bytes([wav_bytes[68], wav_bytes[69], wav_bytes[70], wav_bytes[71]]),
            (2 * TRUESPEECH_PACKET_BYTES) as u32
        );
        assert_eq!(
            wav_bytes.len(),
            WAV_HEADER_SIZE + 2 * TRUESPEECH_PACKET_BYTES
        );

        let mut expected_packets = [0u8; 2 * TRUESPEECH_PACKET_BYTES];
        let mut encoder = TrueSpeechContext::new();
        assert_eq!(
            encoder.encode_samples_into(&pcm_samples, &mut expected_packets),
            Ok(expected_packets.len())
        );
        assert_eq!(&wav_bytes[WAV_HEADER_SIZE..], &expected_packets);
    }
}
