//! 64-bit Rust implementation of the TrueSpeech encoder.
//!
//! The encoder is exposed both as a stateful Rust API and through the legacy
//! `TrueSpeech_*` C ABI expected by native callers.

use std::{ffi::c_void, ptr};

pub mod encoder;
mod wav;

pub use encoder::{
    EncodeBufferError, FrameParams, TRUESPEECH_FRAME_SAMPLES, TRUESPEECH_PACKET_BYTES,
    TRUESPEECH_SAMPLE_RATE, TRUESPEECH_SUBFRAME_SAMPLES, TRUESPEECH_VERSION, TrueSpeechContext,
};
pub use wav::{TrueSpeechWavError, encode_truespeech_wav, write_truespeech_wav};

/// Returns the version reported by the original TrueSpeech encoder.
#[unsafe(no_mangle)]
pub extern "system" fn TrueSpeech_Version() -> u32 {
    TrueSpeechContext::version()
}

/// Creates an opaque encoder state for the PCM/TrueSpeech format pair.
///
/// # Safety
/// Both pointers must contain a readable format tag. The TrueSpeech format
/// must contain at least eleven readable `i16` values.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn TrueSpeech_Init(
    input_format: *const i16,
    output_format: *const i16,
) -> *mut c_void {
    if input_format.is_null() || output_format.is_null() {
        return ptr::null_mut();
    }

    let (input_tag, output_tag) = unsafe {
        (
            ptr::read_unaligned(input_format),
            ptr::read_unaligned(output_format),
        )
    };
    let ts = match (input_tag, output_tag) {
        (1, 0x22) => output_format,
        (0x22, 1) => input_format,
        _ => return ptr::null_mut(),
    };
    let valid_truespeech_descriptor =
        unsafe { ptr::read_unaligned(ts.add(9)) == 1 && ptr::read_unaligned(ts.add(10)) == 240 };
    if !valid_truespeech_descriptor {
        return ptr::null_mut();
    }
    Box::into_raw(Box::new(TrueSpeechContext::new())).cast()
}

/// Encodes one 240-sample PCM frame into eight little-endian 32-bit words.
///
/// # Safety
/// `state` must be a live pointer returned by [`TrueSpeech_Init`]. `pcm` must
/// reference 480 readable bytes; it may be unaligned. `packet` must reference
/// eight writable `u32` values. The state, input, and output memory ranges must
/// not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn TrueSpeech_Encod(
    state: *mut c_void,
    pcm: *const i16,
    packet: *mut u32,
) -> u32 {
    if state.is_null() || pcm.is_null() || packet.is_null() {
        return 1;
    }

    unsafe {
        let context = &mut *state.cast::<TrueSpeechContext>();
        if pcm.is_aligned() {
            let samples = &*pcm.cast::<[i16; TRUESPEECH_FRAME_SAMPLES]>();
            let packet_bytes = &mut *packet.cast::<[u8; TRUESPEECH_PACKET_BYTES]>();
            context.encode_frame(samples, packet_bytes);
        } else {
            let mut aligned_samples = [0i16; TRUESPEECH_FRAME_SAMPLES];
            ptr::copy_nonoverlapping(
                pcm.cast::<u8>(),
                aligned_samples.as_mut_ptr().cast::<u8>(),
                TRUESPEECH_FRAME_SAMPLES * std::mem::size_of::<i16>(),
            );
            let packet_bytes = &mut *packet.cast::<[u8; TRUESPEECH_PACKET_BYTES]>();
            context.encode_frame(&aligned_samples, packet_bytes);
        }
    }
    0
}

/// Clears the predictor state of an initialized encoder.
///
/// # Safety
/// `state` must be a live pointer returned by [`TrueSpeech_Init`].
#[unsafe(no_mangle)]
pub unsafe extern "system" fn TrueSpeech_Reset(state: *mut c_void) -> u32 {
    if state.is_null() {
        return 1;
    }
    unsafe { (&mut *state.cast::<TrueSpeechContext>()).reset() };
    0
}

/// Releases an encoder state created by [`TrueSpeech_Init`].
///
/// # Safety
/// `state` must be null or a live pointer returned by [`TrueSpeech_Init`],
/// and must not be used after this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn TrueSpeech_Term(state: *mut c_void) -> u32 {
    if !state.is_null() {
        unsafe { drop(Box::from_raw(state.cast::<TrueSpeechContext>())) };
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_accepts_pcm_tag_without_extended_fields() {
        let pcm = [1i16];
        let ts = [0x22i16, 1, 8000, 0, 1067, 0, 32, 1, 14, 1, 240];
        unsafe {
            let state = TrueSpeech_Init(pcm.as_ptr(), ts.as_ptr());
            assert!(!state.is_null());
            assert_eq!(TrueSpeech_Term(state), 0);
        }
    }

    #[test]
    fn encoder_accepts_unaligned_pcm_and_matches_the_rust_api() {
        let pcm_format = [1i16, 1, 8_000, 0, 16_000, 0, 2, 16, 0, 0, 0];
        let truespeech_format = [0x22i16, 1, 8_000, 0, 1_067, 0, 32, 1, 14, 1, 240];
        let mut input_samples = [0i16; TRUESPEECH_FRAME_SAMPLES];
        for (index, sample) in input_samples.iter_mut().enumerate() {
            *sample = ((index as i32 * 97 % 30_001) - 15_000) as i16;
        }

        let mut unaligned_pcm = [0u8; TRUESPEECH_FRAME_SAMPLES * 2 + 1];
        for (index, sample) in input_samples.iter().enumerate() {
            let offset = 1 + index * 2;
            unaligned_pcm[offset..offset + 2].copy_from_slice(&sample.to_le_bytes());
        }

        let context = unsafe { TrueSpeech_Init(pcm_format.as_ptr(), truespeech_format.as_ptr()) };
        assert!(!context.is_null());
        let mut packet_words = [0u32; TRUESPEECH_PACKET_BYTES / 4];
        let status = unsafe {
            TrueSpeech_Encod(
                context,
                unaligned_pcm.as_ptr().add(1).cast::<i16>(),
                packet_words.as_mut_ptr(),
            )
        };
        assert_eq!(status, 0);
        assert_eq!(unsafe { TrueSpeech_Term(context) }, 0);
        let mut expected_context = TrueSpeechContext::new();
        let mut expected_packet = [0u8; TRUESPEECH_PACKET_BYTES];
        expected_context.encode_frame(&input_samples, &mut expected_packet);
        let actual_packet = packet_words.map(u32::to_le_bytes).concat();
        assert_eq!(actual_packet, expected_packet);
    }
}
