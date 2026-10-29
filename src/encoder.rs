//! Rust implementation of the DSP Group TrueSpeech 8.5 kbps encoder (version 1.03, `tsd32.dll`).
//! Encoder tables and control flow were reconstructed by comparing DLL traces and fixed data.
//!
//! Standard frame format:
//! - Sample rate: 8,000 Hz, mono.
//! - PCM frame: 240 signed 16-bit samples (480 bytes, 30 ms).
//! - Encoded frame: 32 bytes (256 bits).

#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]

pub const TRUESPEECH_SAMPLE_RATE: u32 = 8_000;
pub const TRUESPEECH_FRAME_SAMPLES: usize = 240;
pub const TRUESPEECH_SUBFRAME_SAMPLES: usize = 60;
const TRUESPEECH_PITCH_WINDOW_SAMPLES: usize = TRUESPEECH_FRAME_SAMPLES / 2;
pub const TRUESPEECH_PACKET_BYTES: usize = 32;
pub const TRUESPEECH_VERSION: u32 = 0x0100_0000;

/// Encoding failed because the packet buffer is smaller than the required size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EncodeBufferError {
    PacketLengthOverflow,
    OutputTooSmall { required: usize, provided: usize },
}

impl core::fmt::Display for EncodeBufferError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PacketLengthOverflow => {
                formatter.write_str("encoded packet size overflows usize")
            }
            Self::OutputTooSmall { required, provided } => write!(
                formatter,
                "output buffer is too small: {required} bytes required, {provided} provided"
            ),
        }
    }
}

impl core::error::Error for EncodeBufferError {}

// ============================================================================
// Constants and codebooks (DSP lookup tables).
// ============================================================================

/// Number of bits allocated to each of the eight LPC reflection coefficients.
const CB_BITS: [u8; 8] = [5, 5, 4, 4, 4, 3, 3, 3];

/// DAT_73b41050: coefficient codebook 0 (32 entries, 5 bits).
const TS_CB_0: [i16; 32] = [
    -32192, -31900, -31538, -31139, -30715, -30242, -29737, -29196, -28591, -27934, -27170, -26353,
    -25471, -24455, -23220, -21806, -20086, -18166, -16092, -13876, -11463, -8749, -5674, -1901,
    2415, 6858, 10732, 14367, 17913, 21610, 25539, 29621,
];

/// DAT_73b41090: coefficient codebook 1 (32 entries, 5 bits).
const TS_CB_1: [i16; 32] = [
    -24731, -19093, -14973, -11407, -8168, -5196, -2532, -167, 2139, 4358, 6482, 8522, 10441,
    12280, 14054, 15762, 17375, 18875, 20294, 21607, 22832, 23971, 25068, 26105, 27092, 27994,
    28830, 29613, 30315, 30960, 31578, 32165,
];

/// DAT_73b410d0: coefficient codebook 2 (16 entries, 4 bits).
const TS_CB_2: [i16; 16] = [
    -26888, -23628, -20667, -17837, -15183, -12596, -10129, -7650, -5133, -2496, 247, 3104, 6273,
    9882, 14187, 19808,
];

/// DAT_73b410f0: coefficient codebook 3 (16 entries, 4 bits).
const TS_CB_3: [i16; 16] = [
    -14764, -8462, -4182, -620, 2410, 5183, 7803, 10284, 12662, 14985, 17311, 19618, 21887, 24144,
    26392, 28557,
];

/// DAT_73b41110: coefficient codebook 4 (16 entries, 4 bits).
const TS_CB_4: [i16; 16] = [
    -21529, -17496, -14308, -11482, -8946, -6700, -4574, -2536, -472, 1615, 3767, 6072, 8618,
    11659, 15266, 19961,
];

/// Coefficient codebook 5 (8 entries, 3 bits; initialized by `TrueSpeech_Init`).
const TS_CB_5: [i16; 8] = [-10981, -3794, 1070, 5063, 8800, 12571, 16606, 21381];

/// Coefficient codebook 6 (8 entries, 3 bits; initialized by `TrueSpeech_Init`).
const TS_CB_6: [i16; 8] = [-19120, -14299, -9856, -5737, -1917, 1874, 6161, 11800];

/// Coefficient codebook 7 (8 entries, 3 bits; initialized by `TrueSpeech_Init`).
const TS_CB_7: [i16; 8] = [-12560, -6919, -2373, 1606, 5365, 9215, 13679, 19085];

const TS_CODEBOOKS: [&[i16]; 8] = [
    &TS_CB_0, &TS_CB_1, &TS_CB_2, &TS_CB_3, &TS_CB_4, &TS_CB_5, &TS_CB_6, &TS_CB_7,
];

/// DAT_73b41228 (offset 0x140): combinatorial table for packing pulse positions.
const TS_PULSE_COMBINADIC_140: [i16; 120] = [
    3654, 3276, 2925, 2600, 2300, 2024, 1771, 1540, 1330, 1140, 969, 816, 680, 560, 455, 364, 286,
    220, 165, 120, 84, 56, 35, 20, 10, 4, 1, 0, 0, 0, 406, 378, 351, 325, 300, 276, 253, 231, 210,
    190, 171, 153, 136, 120, 105, 91, 78, 66, 55, 45, 36, 28, 21, 15, 10, 6, 3, 1, 0, 0, 29, 28,
    27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2,
    1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
];

/// DAT_73b41318 (offset 0x230): final LPC scaling coefficients (0.994^k).
const TS_LPC_SCALE_230: [i16; 8] = [32571, 32376, 32182, 31989, 31797, 31606, 31416, 31228];

/// DAT_73b41328 (offset 0x51A): bandwidth expansion coefficients for perceptual weighting (0.8^k).
const TS_PERCEPTUAL_GAMMA_51A: [i16; 8] = [26214, 20972, 16777, 13422, 10737, 8590, 6872, 5498];

/// DAT_73b41558 (offset 0x52A): autocorrelation lag window.
const TS_LAG_WINDOW_52A: [i16; 8] = [32756, 32721, 32663, 32582, 32478, 32351, 32201, 32030];

/// DAT_73b41356 (reversed near offset 0x542): pulse-gain thresholds (16 levels).
const TS_PULSE_GAIN_LEVELS_542: [i16; 16] = [
    8, 16, 24, 40, 64, 100, 160, 256, 404, 644, 1024, 1624, 2580, 4096, 6500, 10320,
];

/// Offset 0x53A: pulse sign and magnitude factors (positive/negative, small/large).
const TS_PULSE_SIGN_SCALE_53A: [i16; 4] = [0x2000, 0x6000, -0x2000, -0x6000];

/// DAT_73b415e8 (offset 0x240): two-tap pitch-filter coefficients and energy terms (25 × 5 entries).
/// The first two values in each row (b0, b1) are the two-tap filter coefficients (`ts_order2_coeffs`).
/// The remaining values (-b0^2/2, -b1^2/2, -b0*b1) are used by the energy scoring routine `FUN_73b43288`.
const TS_PITCH_FILTER_240: [[i16; 5]; 25] = [
    [-4817, 21049, -1416, -27042, 12376],
    [21745, -6999, -28859, -2989, 18577],
    [9760, -4546, -5814, -1261, 5416],
    [2518, 11328, -387, -7832, -3482],
    [-4171, 11232, -1062, -7700, 5718],
    [16353, 13113, -16323, -10495, -26177],
    [17455, -6402, -18595, -2501, 13640],
    [17496, -1569, -18683, -150, 3350],
    [-3535, 17371, -763, -18417, 7495],
    [15792, -2299, -15222, -323, 4433],
    [20347, -261, -25269, -4, 649],
    [9901, 3292, -5984, -661, -3978],
    [13250, 1849, -10716, -209, -2991],
    [4798, 17314, -1405, -18296, -10140],
    [7135, 7998, -3107, -3905, -6966],
    [529, 1942, -17, -230, -125],
    [10987, 5695, -7368, -1979, -7638],
    [1293, 14904, -102, -13557, -2352],
    [3358, 3448, -688, -726, -1414],
    [5391, 13126, -1774, -10516, -8639],
    [14500, 2941, -12832, -528, -5206],
    [11613, 8159, -8231, -4063, -11566],
    [6583, 10274, -2645, -6442, -8256],
    [3481, 7954, -740, -3861, -3380],
    [6476, 3302, -2560, -665, -2610],
];

/// DAT_73b41378 (offset 0x33A): symmetric 240-sample Hamming window.
const TS_HAMMING_WINDOW_33A: [i16; 240] = [
    2621, 2627, 2642, 2668, 2705, 2751, 2809, 2876, 2954, 3041, 3139, 3247, 3365, 3493, 3631, 3778,
    3935, 4102, 4278, 4463, 4657, 4861, 5073, 5294, 5524, 5761, 6008, 6262, 6524, 6794, 7071, 7356,
    7648, 7947, 8252, 8565, 8883, 9208, 9538, 9874, 10215, 10562, 10913, 11270, 11630, 11995,
    12364, 12736, 13112, 13491, 13873, 14258, 14645, 15034, 15425, 15817, 16211, 16606, 17001,
    17398, 17794, 18190, 18586, 18981, 19375, 19769, 20160, 20550, 20938, 21324, 21707, 22088,
    22466, 22840, 23210, 23577, 23940, 24298, 24652, 25001, 25345, 25684, 26017, 26345, 26666,
    26982, 27291, 27593, 27888, 28177, 28458, 28731, 28997, 29256, 29506, 29748, 29982, 30207,
    30424, 30631, 30830, 31020, 31201, 31372, 31534, 31686, 31829, 31961, 32084, 32197, 32300,
    32393, 32476, 32548, 32611, 32663, 32704, 32735, 32756, 32767, 32767, 32756, 32735, 32704,
    32663, 32611, 32548, 32476, 32393, 32300, 32197, 32084, 31961, 31829, 31686, 31534, 31372,
    31201, 31020, 30830, 30631, 30424, 30207, 29982, 29748, 29506, 29256, 28997, 28731, 28458,
    28177, 27888, 27593, 27291, 26982, 26666, 26345, 26017, 25684, 25345, 25001, 24652, 24298,
    23940, 23577, 23210, 22840, 22466, 22088, 21707, 21324, 20938, 20550, 20160, 19769, 19375,
    18981, 18586, 18190, 17794, 17398, 17001, 16606, 16211, 15817, 15425, 15034, 14645, 14258,
    13873, 13491, 13112, 12736, 12364, 11995, 11630, 11270, 10913, 10562, 10215, 9874, 9538, 9208,
    8883, 8565, 8252, 7947, 7648, 7356, 7071, 6794, 6524, 6262, 6008, 5761, 5524, 5294, 5073, 4861,
    4657, 4463, 4278, 4102, 3935, 3778, 3631, 3493, 3365, 3247, 3139, 3041, 2954, 2876, 2809, 2751,
    2705, 2668, 2642, 2627, 2621,
];

// ============================================================================
// Encoder parameters stored in the DLL-compatible frame layout.
// Corresponds to offsets 0xCF4 .. 0xD32 in the tsd32.dll state block.
// ============================================================================

#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// Quantized parameters produced for one TrueSpeech frame.
pub struct FrameParams {
    /// 0xCF4..0xD03: indices for the eight reflection-coefficient codebooks.
    pub lpc_indices: [i16; 8],
    /// 0xD04..0xD07: open-loop pitch lags for both frame halves: [half0, half1].
    pub base_pitch_lags: [i16; 2],
    /// 0xD08..0xD0F: fractional-filter and adaptive-gain indices for four subframes.
    pub adaptive_indices: [i16; 4],
    /// 0xD10..0xD17: pulse-gain level indices for four subframes.
    pub pulse_gain_indices: [i16; 4],
    /// 0xD18..0xD27: combinatorial pulse-position code (27 bits per subframe).
    pub pulse_pos_codes: [u32; 4],
    /// 0xD28..0xD2F: signs and amplitudes for seven pulses (14 bits per subframe).
    pub pulse_signs_packed: [u16; 4],
    /// 0xD30: interpolation flag (1 bit).
    pub interp_flag: u16,
}

// ============================================================================
// Complete TrueSpeech state (corresponds to the 0x0D34-byte state block in `tsd32.dll`).
// ============================================================================

#[derive(Clone)]
/// Stateful TrueSpeech encoder for mono 16-bit PCM at 8 kHz.
///
/// Keep one context for each audio stream and reuse it for consecutive frames.
pub struct TrueSpeechContext {
    /// 0x33A: 240-sample LPC analysis window.
    hamming_window: [i16; 240],
    /// 0x604: excitation history (146 samples = 0x92 shorts).
    excitation_history: [i16; 146],
    /// 0x728: previous LPC coefficients (8 shorts = 16 bytes).
    prev_lpc: [i16; 8],
    /// 0x738: analysis-window buffer (240 samples).
    windowed_buf: [i16; 240],
    /// 0x918: active-frame buffer (240 samples).
    working_frame: [i16; 240],
    /// 0xAF8: overlap buffer from the previous frame (60 samples).
    overlap_buf: [i16; 60],
    /// 0xB70: weighted-signal history for pitch search (146 samples).
    weighted_pitch_history: [i16; 146],
    /// 0xC94: perceptual-weighting numerator filter state (8 shorts).
    perceptual_fir_state: [i16; 8],
    /// 0xCA4: perceptual-weighting denominator filter state (8 shorts).
    perceptual_iir_state: [i16; 8],
    /// 0xCB4: encoder synthesis-filter ringing state (8 shorts).
    encoder_synth_state: [i16; 8],
    /// 0xCF4..0xD32: parameters for the current frame.
    pub params: FrameParams,
}

impl Default for TrueSpeechContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TrueSpeechContext {
    /// Corresponds to `TrueSpeech_Init` (`0x73b41700`).
    pub fn new() -> Self {
        Self {
            hamming_window: TS_HAMMING_WINDOW_33A,
            excitation_history: [0; 146],
            prev_lpc: [0; 8],
            windowed_buf: [0; 240],
            working_frame: [0; 240],
            overlap_buf: [0; 60],
            weighted_pitch_history: [0; 146],
            perceptual_fir_state: [0; 8],
            perceptual_iir_state: [0; 8],
            encoder_synth_state: [0; 8],
            params: FrameParams::default(),
        }
    }

    /// Creates an encoder when the descriptors identify PCM and TrueSpeech.
    ///
    /// The TrueSpeech descriptor must contain at least eleven `i16` values;
    /// values 9 and 10 must be `1` (revision) and `240` (samples per block).
    /// The PCM descriptor only needs its format tag. Returns `None` when the
    /// tags or required TrueSpeech fields do not match.
    pub fn init_from_waveformat(fmt_a: &[i16], fmt_b: &[i16]) -> Option<Self> {
        if fmt_a.is_empty() || fmt_b.is_empty() {
            return None;
        }
        let ts_fmt = if fmt_a[0] == 1 {
            if fmt_b[0] != 0x22 {
                return None;
            }
            fmt_b
        } else if fmt_a[0] == 0x22 && fmt_b[0] == 1 {
            fmt_a
        } else {
            return None;
        };
        if ts_fmt.len() >= 11 && ts_fmt[9] == 1 && ts_fmt[10] == 0xF0 {
            Some(Self::new())
        } else {
            None
        }
    }

    /// Returns the codec version, encoded as `0x01000000`.
    pub const fn version() -> u32 {
        TRUESPEECH_VERSION
    }

    /// Resets predictor and history state to its initial values.
    pub fn reset(&mut self) {
        self.excitation_history.fill(0);
        self.prev_lpc.fill(0);
        self.windowed_buf.fill(0);
        self.working_frame.fill(0);
        self.overlap_buf.fill(0);
        self.weighted_pitch_history.fill(0);
        self.perceptual_fir_state.fill(0);
        self.perceptual_iir_state.fill(0);
        self.encoder_synth_state.fill(0);
        self.params = FrameParams::default();
    }

    /// Returns the required packet-buffer size for PCM samples, including a padded partial frame.
    pub const fn encoded_packet_bytes(sample_count: usize) -> Option<usize> {
        let frame_count = sample_count.div_ceil(TRUESPEECH_FRAME_SAMPLES);
        frame_count.checked_mul(TRUESPEECH_PACKET_BYTES)
    }

    /// Encodes a PCM slice into caller-provided storage without heap allocation.
    ///
    /// Each frame contains 240 samples. Missing samples in the final frame are zero-padded.
    /// Returns the number of packet bytes written. If the buffer is too small, neither the
    /// encoder state nor the output buffer is modified.
    pub fn encode_samples_into(
        &mut self,
        samples: &[i16],
        packet_output: &mut [u8],
    ) -> Result<usize, EncodeBufferError> {
        let required_bytes = Self::encoded_packet_bytes(samples.len())
            .ok_or(EncodeBufferError::PacketLengthOverflow)?;
        if packet_output.len() < required_bytes {
            return Err(EncodeBufferError::OutputTooSmall {
                required: required_bytes,
                provided: packet_output.len(),
            });
        }

        let (packet_frames, trailing_bytes) =
            packet_output[..required_bytes].as_chunks_mut::<TRUESPEECH_PACKET_BYTES>();
        debug_assert!(trailing_bytes.is_empty());

        let (complete_input_frames, partial_input_frame) =
            samples.as_chunks::<TRUESPEECH_FRAME_SAMPLES>();
        for (input_frame, packet) in complete_input_frames.iter().zip(packet_frames.iter_mut()) {
            self.encode_frame(input_frame, packet);
        }

        if !partial_input_frame.is_empty() {
            let mut padded_frame = [0i16; TRUESPEECH_FRAME_SAMPLES];
            padded_frame[..partial_input_frame.len()].copy_from_slice(partial_input_frame);
            let partial_packet = &mut packet_frames[complete_input_frames.len()];
            self.encode_frame(&padded_frame, partial_packet);
        }

        Ok(required_bytes)
    }

    /// Encodes one frame and returns its 32-byte packet.
    ///
    /// Reuse the same context for consecutive frames in a stream. The input
    /// must contain 240 mono, signed 16-bit PCM samples at 8 kHz.
    ///
    pub fn encode_frame_to_packet(
        &mut self,
        pcm_samples: &[i16; TRUESPEECH_FRAME_SAMPLES],
    ) -> [u8; TRUESPEECH_PACKET_BYTES] {
        let mut packet = [0; TRUESPEECH_PACKET_BYTES];
        self.encode_frame(pcm_samples, &mut packet);
        packet
    }

    /// Corresponds to `TrueSpeech_Encod` (`0x73b42891`).
    /// Encodes one 240-sample mono PCM frame into a 32-byte packet.
    ///
    /// The caller supplies both fixed-size arrays. The packet is written in
    /// the codec's byte order, and the encoder state advances by one frame.
    pub fn encode_frame(
        &mut self,
        pcm_samples: &[i16; TRUESPEECH_FRAME_SAMPLES],
        packet_bytes: &mut [u8; TRUESPEECH_PACKET_BYTES],
    ) {
        // 1. Scale the signal and update the overlap buffer (FUN_73b422f7).
        scale_and_shift_input(
            &mut self.windowed_buf,
            &mut self.working_frame,
            pcm_samples,
            &mut self.overlap_buf,
        );

        // 2. Apply the Hamming window (FUN_73b42831).
        apply_hamming_window(&mut self.windowed_buf, &self.hamming_window);

        // 3. Compute autocorrelation (FUN_73b43640).
        let mut autocorrelation = [0i16; 9];
        compute_autocorrelation(&mut autocorrelation, &self.windowed_buf);

        // 4. Apply the autocorrelation lag window (FUN_73b42862).
        let mut lagged_autocorrelation = [0i16; 8];
        lagged_autocorrelation.copy_from_slice(&autocorrelation[1..9]);
        apply_lag_window(&mut lagged_autocorrelation, &TS_LAG_WINDOW_52A);
        autocorrelation[1..9].copy_from_slice(&lagged_autocorrelation);

        // 5. Use Levinson-Durbin to calculate reflection coefficients (FUN_73b43700).
        let mut reflection_coefficients = [0i16; 8];
        levinson_durbin(&mut reflection_coefficients, &autocorrelation);

        // 6. Quantize and dequantize the reflection coefficients (FUN_73b437ff + FUN_73b4388b).
        quantize_reflection_coeffs(&mut self.params.lpc_indices, &reflection_coefficients);
        dequantize_reflection_coeffs(&mut reflection_coefficients, &self.params.lpc_indices);

        // 7. Convert reflection coefficients to LPC filter coefficients (FUN_73b438ba).
        let mut current_lpc_coefficients = [0i16; 8];
        reflection_to_lpc(
            &mut current_lpc_coefficients,
            &reflection_coefficients,
            &TS_LPC_SCALE_230,
        );

        // 8. Interpolate LPC coefficients for four subframes and check stability (FUN_73b43ae1).
        let mut subframe_lpc_coefficients = [[0i16; 8]; 4];
        self.params.interp_flag = interpolate_lpc_encoder(
            &mut subframe_lpc_coefficients,
            &current_lpc_coefficients,
            &self.prev_lpc,
        );

        // 9. Apply perceptual weighting to the complete frame (FUN_73b423b1).
        let mut perceptual_residual = [0i16; TRUESPEECH_FRAME_SAMPLES];
        perceptual_weighting_filter(
            &mut self.working_frame,
            &mut perceptual_residual,
            &mut subframe_lpc_coefficients,
            &mut self.perceptual_fir_state,
            &mut self.perceptual_iir_state,
            &TS_PERCEPTUAL_GAMMA_51A,
        );

        // 10. Process four subframes (60 samples per subframe).
        let mut impulse_response = [0i16; 30];
        let mut pulse_energy: i16 = 0;

        let residual_subframes = perceptual_residual
            .as_chunks::<TRUESPEECH_SUBFRAME_SAMPLES>()
            .0;
        for subframe_index in 0..4 {
            // Update the open-loop pitch search once per half-frame.
            if subframe_index % 2 == 0 {
                let pitch_halves = self
                    .working_frame
                    .as_chunks::<TRUESPEECH_PITCH_WINDOW_SAMPLES>()
                    .0;
                let pitch_target = &pitch_halves[subframe_index / 2];
                let pitch_lag = open_loop_pitch_search(pitch_target, &self.weighted_pitch_history);
                self.params.base_pitch_lags[subframe_index / 2] = pitch_lag as i16;
            }

            let lpc_coefficients = &subframe_lpc_coefficients[subframe_index];
            let target_subframes = self
                .working_frame
                .as_chunks_mut::<TRUESPEECH_SUBFRAME_SAMPLES>()
                .0;
            let target_subframe = &mut target_subframes[subframe_index];

            // Subtract zero-input response ringing before codebook search.
            subtract_ringing(target_subframe, lpc_coefficients, &self.encoder_synth_state);

            // Calculate impulse response and energy for the first three subframes.
            if subframe_index < 3 {
                pulse_energy =
                    impulse_response_energy(&mut impulse_response, lpc_coefficients) as i16;
            }

            // Search the adaptive codebook for the best excitation vector.
            let mut adaptive_vector = [0i16; TRUESPEECH_SUBFRAME_SAMPLES];
            let base_lag = self.params.base_pitch_lags[subframe_index / 2];
            adaptive_codebook_search(
                target_subframe,
                &mut adaptive_vector,
                &self.excitation_history,
                &residual_subframes[subframe_index],
                lpc_coefficients,
                &impulse_response[1..30],
                base_lag,
                &mut self.params,
                subframe_index,
            );

            // Select fixed-codebook MP-MLQ pulses and write the frame indices.
            fixed_codebook_mpmlq_search(
                target_subframe,
                &impulse_response,
                pulse_energy,
                &mut self.params,
                subframe_index,
            );

            // Update excitation history and weighted state for the next frame.
            update_excitation_history(
                target_subframe,
                &mut self.excitation_history,
                &adaptive_vector,
            );
            update_weighted_synthesis_history(
                &mut self.weighted_pitch_history,
                lpc_coefficients,
                target_subframe,
                &mut self.encoder_synth_state,
            );
        }

        // Save LPC coefficients for the next frame.
        self.prev_lpc = current_lpc_coefficients;

        // 11. Pack parameters into 32 bytes (bit packing at the end of TrueSpeech_Encod).
        pack_bitstream(&self.params, packet_bytes);
    }
}

// ============================================================================
// Internal DSP helper functions.
// ============================================================================

/// Corresponds to `FUN_73b43b8c`: counts leading sign/zero bits to normalize a 32-bit value.
#[inline]
fn normalize_u32(val: &mut u32) -> i32 {
    let mut shifts = 0;
    if *val != 0 {
        while (*val & 0xC000_0000) == 0 {
            *val <<= 1;
            shifts += 1;
        }
    }
    shifts
}

#[inline]
fn wrapping_abs_i32(value: i32) -> i32 {
    (value ^ (value >> 31)).wrapping_sub(value >> 31)
}

/// Corresponds to `FUN_73b422f7`: scales input and updates the overlap and working-frame buffers.
fn scale_and_shift_input(
    windowed_samples: &mut [i16; 240],
    working_frame: &mut [i16; 240],
    pcm_samples: &[i16; 240],
    overlap_samples: &mut [i16; 60],
) {
    let mut max_abs: i32 = 0;
    for &sample in pcm_samples {
        let absolute_sample = (sample as i32).abs();
        if absolute_sample > max_abs {
            max_abs = absolute_sample;
        }
    }

    let mut shift = u8::from(max_abs > 0x1000);
    if max_abs > 0x2000 {
        shift += 1;
    }
    if max_abs > 0x4000 {
        shift += 1;
    }

    for (windowed_sample, &pcm_sample) in windowed_samples.iter_mut().zip(pcm_samples) {
        *windowed_sample = pcm_sample >> shift;
    }

    // Move the previous-frame overlap, then save the final 60 samples of the current input.
    working_frame[..60].copy_from_slice(overlap_samples);
    overlap_samples.copy_from_slice(&pcm_samples[180..240]);

    working_frame[60..240].copy_from_slice(&pcm_samples[..180]);
}

/// Corresponds to `FUN_73b42831`: multiplies the signal by a 240-sample Hamming window.
fn apply_hamming_window(buf: &mut [i16; 240], window: &[i16; 240]) {
    for i in 0..240 {
        buf[i] = (((window[i] as i32) * (buf[i] as i32) + 0x4000) >> 15) as i16;
    }
}

/// Corresponds to `FUN_73b42862`: multiplies autocorrelation coefficients by the 8-value lag window.
fn apply_lag_window(buf: &mut [i16; 8], window: &[i16; 8]) {
    for i in 0..8 {
        buf[i] = (((window[i] as i32) * (buf[i] as i32) + 0x4000) >> 15) as i16;
    }
}

/// Corresponds to `FUN_73b43640`: calculates the nine autocorrelation values R[0..8].
fn compute_autocorrelation(autocorr: &mut [i16; 9], signal: &[i16; 240]) {
    let mut sum0: u32 = 0;
    for &s in signal.iter() {
        let v = s as i32;
        sum0 = sum0.wrapping_add((v * v) as u32);
    }

    let shift = normalize_u32(&mut sum0);
    let mut adjusted = (sum0 as i32).wrapping_add((sum0 as i32) >> 12);
    if adjusted < 0 {
        adjusted = 0x7FFF_0000;
    }
    autocorr[0] = ((adjusted as u32).wrapping_add(0x8000) >> 16) as i16;

    for lag in 1..9 {
        let mut sum: u32 = 0;
        let count = 240 - lag;
        for i in 0..count {
            let prod = (signal[i] as i32) * (signal[i + lag] as i32);
            sum = sum.wrapping_add(prod as u32);
        }
        let shifted = sum.wrapping_shl((shift as u32) & 0x1F);
        autocorr[lag] = (shifted.wrapping_add(0x8000) >> 16) as i16;
    }
}

/// Corresponds to `FUN_73b43700`: Schur / Levinson-Durbin algorithm for eight reflection coefficients.
fn levinson_durbin(reflection: &mut [i16; 8], autocorr: &[i16; 9]) {
    let mut e_short = autocorr[0];
    let mut e_scaled = (autocorr[0] as i32) << 15;
    let mut b_vec = [0i16; 8];
    let mut f_vec = [0i16; 8];

    for i in 0..8 {
        b_vec[i] = autocorr[i + 1];
        f_vec[i] = autocorr[i + 1];
        reflection[i] = 0;
    }

    for step in 0..8 {
        let num = (b_vec[0] as i32) << 15;
        if e_scaled <= num.abs() || e_short == 0 {
            return;
        }
        let k = ((num / (e_short as i32)) as i16).wrapping_neg();
        reflection[step] = k;
        let k_i32 = k as i32;
        e_scaled = e_scaled.wrapping_add(k_i32 * (b_vec[0] as i32));
        e_short = (e_scaled.wrapping_add(0x4000) >> 15) as i16;

        let rem = 8 - step;
        for j in 0..rem {
            let next_b = if j + 1 < 8 { b_vec[j + 1] } else { 0 };
            let cur_f = f_vec[j];
            let prod_f = (cur_f as i32) * k_i32;
            f_vec[j] = (((next_b as i32) * k_i32 + 0x4000 + (cur_f as i32) * 0x8000) >> 15) as i16;
            b_vec[j] = ((prod_f + 0x4000 + (next_b as i32) * 0x8000) >> 15) as i16;
        }
    }
}

/// Corresponds to `FUN_73b437ff`: quantizes reflection coefficients using the nearest value in eight codebooks.
fn quantize_reflection_coeffs(indices: &mut [i16; 8], reflection: &[i16; 8]) {
    for i in 0..8 {
        let cb = TS_CODEBOOKS[i];
        let count = 1usize << CB_BITS[i];
        let target = reflection[i] as i32;
        let mut best_idx = 0i16;
        let mut best_err = i32::MAX;

        for (idx, &cand) in cb.iter().take(count).enumerate() {
            let err = (target - (cand as i32)).abs();
            if err < best_err {
                best_err = err;
                best_idx = idx as i16;
            }
        }
        indices[i] = best_idx;
    }
}

/// Corresponds to `FUN_73b4388b`: restores reflection-coefficient values from the eight codebooks.
fn dequantize_reflection_coeffs(reflection: &mut [i16; 8], indices: &[i16; 8]) {
    for i in 0..8 {
        let cb = TS_CODEBOOKS[i];
        let idx = (indices[i] as usize).min(cb.len() - 1);
        reflection[i] = cb[idx];
    }
}

/// Corresponds to `FUN_73b438ba`: converts reflection coefficients into LPC coefficients.
fn reflection_to_lpc(lpc: &mut [i16; 8], reflection: &[i16; 8], scale_table: &[i16; 8]) -> i16 {
    lpc[0] = ((8 - (reflection[0] as i32)) >> 3) as i16;
    for m in 1..8 {
        let mut temp = [0i16; 8];
        temp[..m].copy_from_slice(&lpc[..m]);

        let k = reflection[m] as i32;
        for j in 0..m {
            let rev = temp[m - 1 - j] as i32;
            lpc[j] = ((rev * k + 0x4000 + (lpc[j] as i32) * 0x8000) >> 15) as i16;
        }
        lpc[m] = ((8 - k) >> 3) as i16;
    }

    for i in 0..8 {
        lpc[i] = (((scale_table[i] as i32) * (lpc[i] as i32)) >> 15) as i16;
    }
    reflection[0]
}

/// Corresponds to `FUN_73b43a27`: checks interpolated LPC-filter stability with backward Levinson recursion.
fn check_lpc_stability(lpc: &[i16; 8]) -> u16 {
    let mut a = *lpc;
    for m in (0..8).rev() {
        let val = a[m] as i32;
        if val.abs() > 0x0FFF {
            return 0;
        }
        if m == 0 {
            return 1;
        }
        let k = (a[m] << 3) as i32;
        let denom = ((0x3FFF_C000 - k * k) >> 15) as i16 as i32;
        if denom == 0 {
            return 0;
        }
        let mut prev = [0i16; 8];
        prev[..m].copy_from_slice(&a[..m]);
        for j in 0..m {
            let num = (prev[m - 1 - j] as i32) * k + (a[j] as i32) * 0x8000;
            a[j] = (num / denom) as i16;
        }
    }
    0
}

/// Corresponds to `FUN_73b43ae1`: interpolates encoder LPC coefficients while checking filter stability.
fn interpolate_lpc_encoder(
    subframe_lpc: &mut [[i16; 8]; 4],
    current_lpc: &[i16; 8],
    prev_lpc: &[i16; 8],
) -> u16 {
    for i in 0..8 {
        let p = (prev_lpc[i] as i32) * 0x2AAB;
        let c = (current_lpc[i] as i32) * 0x2AAB;
        let sum = p + c;
        subframe_lpc[0][i] = ((sum + 0x4000 + p) >> 15) as i16;
        subframe_lpc[1][i] = ((sum + 0x4000 + c) >> 15) as i16;
    }

    let stable = check_lpc_stability(&subframe_lpc[0]) & check_lpc_stability(&subframe_lpc[1]);
    if stable == 0 {
        subframe_lpc[0] = *prev_lpc;
        subframe_lpc[1] = *current_lpc;
    }
    subframe_lpc[2] = *current_lpc;
    subframe_lpc[3] = *current_lpc;
    stable
}

/// Applies perceptual weighting to the encoder's working frame.
fn perceptual_weighting_filter(
    working_frame: &mut [i16; 240],
    residual_out: &mut [i16; 240],
    subframe_lpc: &mut [[i16; 8]; 4],
    fir_state: &mut [i16; 8],
    iir_state: &mut [i16; 8],
    gamma_table: &[i16; 8],
) {
    let orig_lpc = *subframe_lpc;

    // Expand LPC bandwidth using the gamma table.
    for sf in 0..4 {
        for k in 0..8 {
            subframe_lpc[sf][k] =
                (((gamma_table[k] as i32) * (subframe_lpc[sf][k] as i32) + 0x4000) >> 15) as i16;
        }
    }

    for sf in 0..4 {
        let base = sf * 60;
        for n in 0..60 {
            let x = working_frame[base + n];
            let mut acc = (x as i32) << 12;
            for k in 0..8 {
                acc = acc.wrapping_sub((orig_lpc[sf][k] as i32) * (fir_state[k] as i32));
            }
            residual_out[base + n] = (acc >> 12) as i16;

            for k in (1..8).rev() {
                fir_state[k] = fir_state[k - 1];
            }
            fir_state[0] = x;

            for k in 0..8 {
                acc = acc.wrapping_add((subframe_lpc[sf][k] as i32) * (iir_state[k] as i32));
            }
            working_frame[base + n] = (acc.wrapping_add(0x800) >> 12) as i16;

            for k in (1..8).rev() {
                iir_state[k] = iir_state[k - 1];
            }
            iir_state[0] = (acc >> 12) as i16;
        }
    }
}

/// Corresponds to `FUN_73b424e2`: subtracts zero-input-response ringing from the target signal.
fn subtract_ringing(target_sf: &mut [i16; 60], lpc: &[i16; 8], synth_state: &[i16; 8]) {
    let mut state = *synth_state;
    for n in 0..60 {
        let mut acc: i32 = 0;
        for k in 0..8 {
            acc = acc.wrapping_add((lpc[k] as i32) * (state[k] as i32));
        }
        for k in (1..8).rev() {
            state[k] = state[k - 1];
        }
        state[0] = (acc.wrapping_add(0x800) >> 12) as i16;
        let residual = (target_sf[n] as i32)
            .wrapping_mul(0x1000)
            .wrapping_sub(acc)
            .wrapping_add(0x800);
        target_sf[n] = (residual >> 12) as i16;
    }
}

/// Corresponds to `FUN_73b426e2`: calculates a 30-sample synthesis-filter impulse response and its energy.
fn impulse_response_energy(impulse_resp: &mut [i16; 30], lpc: &[i16; 8]) -> i32 {
    impulse_resp.fill(0);
    impulse_resp[0] = 0x1000;
    let mut state = [0i16; 8];

    for n in 0..30 {
        let mut acc = (impulse_resp[n] as i32) << 12;
        for k in 0..8 {
            acc = acc.wrapping_add((lpc[k] as i32) * (state[k] as i32));
        }
        impulse_resp[n] = (acc.wrapping_add(0x800) >> 12) as i16;
        for k in (1..8).rev() {
            state[k] = state[k - 1];
        }
        state[0] = (acc >> 12) as i16;
    }

    let mut energy: i32 = 0;
    for &h in impulse_resp.iter() {
        let v = h as i32;
        energy = energy.wrapping_add(v * v);
    }
    energy.wrapping_add(0x4000) >> 15
}

/// Corresponds to `FUN_73b4256e`: open-loop search for the fundamental pitch lag.
fn open_loop_pitch_search(target_half: &[i16; 120], history: &[i16; 146]) -> i32 {
    let mut combined = [0i16; 266];
    combined[0..146].copy_from_slice(history);
    combined[146..266].copy_from_slice(target_half);

    let mut best_lag: i32 = 0;
    let mut best_dist: i32 = i32::MAX;

    for lag in 0x14..=0x92 {
        let start = 146usize - (lag as usize);
        let mut dist: i32 = 0;
        let mut aborted = false;

        for block in 0..6 {
            let b_off = block * 20;
            for step in (0..10).step_by(2) {
                dist += ((target_half[b_off + step] as i32)
                    - (combined[start + b_off + step] as i32))
                    .abs();
            }
            if dist >= best_dist {
                aborted = true;
                break;
            }
            for step in (10..20).step_by(2) {
                dist += ((target_half[b_off + step] as i32)
                    - (combined[start + b_off + step] as i32))
                    .abs();
            }
            if dist >= best_dist {
                aborted = true;
                break;
            }
        }

        if !aborted {
            best_dist = dist;
            best_lag = lag;
        }
    }

    // The native epilogue clamps to 0x16..0x8F and then executes ADD EAX, -0x14.
    best_lag.clamp(0x16, 0x8F) - 0x14
}

/// Corresponds to `FUN_73b42d3e`: reconstructs the adaptive-codebook vector selected by closed-loop search.
fn generate_adaptive_vector(
    out_vec: &mut [i16; 60],
    excitation_history: &[i16; 146],
    params: &FrameParams,
    subframe: usize,
) {
    let index = params.adaptive_indices[subframe];
    if index == 0x7F {
        out_vec.fill(0);
        return;
    }

    let base_lag = params.base_pitch_lags[subframe >> 1];
    let mut buffer = [0i16; 206];
    buffer[..146].copy_from_slice(excitation_history);
    let pitch_lag = ((index / 25 + base_lag + 18) as isize).clamp(0, 145) as usize;
    let filter_index = (index % 25) as usize;
    let tap_zero = TS_PITCH_FILTER_240[filter_index][0] as i32;
    let tap_one = TS_PITCH_FILTER_240[filter_index][1] as i32;
    let source_start = 145usize - pitch_lag;

    for (sample_index, output) in out_vec.iter_mut().enumerate() {
        let sample_zero = buffer[source_start + sample_index] as i32;
        let sample_one = buffer[source_start + sample_index + 1] as i32;
        let filtered = tap_zero
            .wrapping_mul(sample_zero)
            .wrapping_add(tap_one.wrapping_mul(sample_one))
            .wrapping_add(0x2000);
        let value = (filtered >> 14) as i16;
        buffer[146 + sample_index] = value;
        *output = value;
    }
}

/// Closed-loop adaptive codebook search used by the encoder.
fn adaptive_codebook_search(
    target_sf: &mut [i16; 60],
    adaptive_vec: &mut [i16; 60],
    excitation_history: &[i16; 146],
    residual_sf: &[i16],
    lpc: &[i16; 8],
    impulse_tail: &[i16],
    base_lag: i16,
    params: &mut FrameParams,
    subframe: usize,
) {
    let mut work_buf = [0i16; 206];
    work_buf[0..146].copy_from_slice(excitation_history);
    work_buf[146..206].copy_from_slice(&residual_sf[0..60]);

    let start_pos = (146 - 18 - (base_lag as isize)).clamp(5, 140) as usize;
    let mut state = [0i16; 8];

    for n in 0..60 {
        let mut acc = (work_buf[start_pos + n] as i32) << 12;
        for k in 0..8 {
            acc = acc.wrapping_add((lpc[k] as i32) * (state[k] as i32));
        }
        work_buf[start_pos + n] = (acc.wrapping_add(0x800) >> 12) as i16;
        for k in (1..8).rev() {
            state[k] = state[k - 1];
        }
        state[0] = (acc >> 12) as i16;
    }

    let mut corr_stats = [0i32; 25];
    let mut cur_pos = start_pos;

    for n in 0..60 {
        let w = work_buf[cur_pos + n] as i32;
        let t = target_sf[n] as i32;
        corr_stats[1] = corr_stats[1].wrapping_add((t * w) >> 2);
        corr_stats[3] = corr_stats[3].wrapping_add((w * w) >> 3);
    }

    for step in 0..5 {
        let prev_pos = cur_pos - 1;
        let prev_sample = work_buf[prev_pos] as i32;
        let mut tail_sum: i32 = 0;
        for k in 0..29.min(impulse_tail.len()) {
            tail_sum =
                tail_sum.wrapping_add((impulse_tail[k] as i32) * (work_buf[cur_pos + k] as i32));
        }
        let high_product = ((tail_sum >> 16) as i16 as i32)
            .wrapping_mul(prev_sample)
            .wrapping_mul(2);
        let mut cross = (((tail_sum & 0xFFFF) * prev_sample) >> 15).wrapping_add(high_product);

        for n in 0..60 {
            cross = cross.wrapping_add(
                ((work_buf[cur_pos + n] as i32) * (work_buf[prev_pos + n] as i32)) >> 3,
            );
        }
        corr_stats[step * 5 + 4] = cross;

        for k in 0..29.min(impulse_tail.len()) {
            let h = impulse_tail[k] as i32;
            let cur = work_buf[cur_pos + k] as i32;
            let filtered = prev_sample
                .wrapping_mul(h)
                .wrapping_add(cur.wrapping_mul(0x1000))
                .wrapping_add(0x800);
            work_buf[cur_pos + k] = (filtered >> 12) as i16;
        }

        let mut c0: i32 = 0;
        let mut c2: i32 = 0;
        for n in 0..60 {
            let w = work_buf[prev_pos + n] as i32;
            let t = target_sf[n] as i32;
            c0 = c0.wrapping_add((t * w) >> 2);
            c2 = c2.wrapping_add((w * w) >> 3);
        }
        corr_stats[step * 5] = c0;
        corr_stats[step * 5 + 2] = c2;
        if step != 4 {
            corr_stats[(step + 1) * 5 + 1] = c0;
            corr_stats[(step + 1) * 5 + 3] = c2;
        }
        cur_pos = prev_pos;
    }

    let mut max_stat: u32 = 0;
    for &v in corr_stats.iter() {
        let a = v.unsigned_abs();
        if a > max_stat {
            max_stat = a;
        }
    }
    let shift = normalize_u32(&mut max_stat);
    let mut norm_stats = [0i16; 25];
    for i in 0..25 {
        norm_stats[i] = (((corr_stats[i] as u32) << ((shift as u32) & 0x1F)) >> 16) as i16;
    }

    let mut best_score: i32 = 0;
    let mut best_idx: i16 = 0x7F;
    let mut candidate_idx: i16 = 0;

    for step in 0..5 {
        let st = &norm_stats[step * 5..step * 5 + 5];
        for row in 0..25 {
            let flt = &TS_PITCH_FILTER_240[row];
            let mut score = 0i32;
            for k in 0..5 {
                score = score.wrapping_add((st[k] as i32) * (flt[k] as i32));
            }
            if score > best_score {
                best_score = score;
                best_idx = candidate_idx;
            }
            candidate_idx += 1;
        }
    }

    params.adaptive_indices[subframe] = best_idx;
    generate_adaptive_vector(adaptive_vec, excitation_history, params, subframe);

    if best_idx != 0x7F {
        let mut syn_state = [0i16; 8];
        for n in 0..60 {
            let mut acc = (adaptive_vec[n] as i32) << 12;
            for k in 0..8 {
                acc = acc.wrapping_add((lpc[k] as i32) * (syn_state[k] as i32));
            }
            for k in (1..8).rev() {
                syn_state[k] = syn_state[k - 1];
            }
            syn_state[0] = (acc >> 12) as i16;
            let residual = (target_sf[n] as i32)
                .wrapping_mul(0x1000)
                .wrapping_sub(acc)
                .wrapping_add(0x800);
            target_sf[n] = (residual >> 12) as i16;
        }
    }
}

/// Corresponds to `FUN_73b42dfe`: MP-MLQ multipulse search (seven pulses: three in the first half and four in the second).
fn fixed_codebook_mpmlq_search(
    target_sf: &mut [i16; 60],
    impulse_resp: &[i16; 30],
    pulse_energy: i16,
    params: &mut FrameParams,
    subframe: usize,
) {
    let mut corr = [0i32; 60];
    let mut max_abs_corr: i32 = 0;
    let mut best_pos: usize = 0;

    for i in 0..30 {
        let mut sum: i32 = 0;
        for k in 0..30 {
            sum = sum.wrapping_add((impulse_resp[k] as i32) * (target_sf[i + k] as i32));
        }
        corr[i] = sum;
        let a = wrapping_abs_i32(sum);
        if a >= max_abs_corr {
            max_abs_corr = a;
            best_pos = i;
        }
    }

    for i in 30..60 {
        let mut sum: i32 = 0;
        let rem = 60 - i;
        for k in 0..rem {
            sum = sum.wrapping_add((impulse_resp[k] as i32) * (target_sf[i + k] as i32));
        }
        corr[i] = sum;
        let a = wrapping_abs_i32(sum);
        if a >= max_abs_corr {
            max_abs_corr = a;
            best_pos = i;
        }
    }

    let scaled_max = max_abs_corr >> 3;
    let mut used = [false; 60];
    let mut gain_idx: usize = 15;
    for g in (0..=15).rev() {
        if scaled_max < (TS_PULSE_GAIN_LEVELS_542[g] as i32) * (pulse_energy as i32) {
            gain_idx = g;
        }
    }

    let base_gain = TS_PULSE_GAIN_LEVELS_542[gain_idx];
    params.pulse_gain_indices[subframe] = gain_idx as i16;

    let mut pulse_positions = [0usize; 7];
    let mut pulse_amps = [0i16; 7];

    used[best_pos] = true;
    let mut sign_code: usize = if corr[best_pos] < 0 { 2 } else { 0 };
    let threshold = (base_gain as i32) * (pulse_energy as i32);
    if (threshold >> 1) <= scaled_max {
        sign_code += 1;
    }

    pulse_positions[0] = best_pos;
    pulse_amps[0] =
        (((TS_PULSE_SIGN_SCALE_53A[sign_code] as i32) * (base_gain as i32)) >> 15) as i16;

    let mut count_first_half = 0usize;
    let mut count_second_half = 0usize;

    for p in 0..6 {
        let pos = pulse_positions[p];
        let amp = pulse_amps[p] as i32;

        if pos < 30 {
            count_first_half += 1;
            for k in 0..(pos + 30).min(60) {
                corr[k] = 0;
            }
            for k in 0..30 {
                let t = target_sf[pos + k] as i32;
                target_sf[pos + k] = (t
                    .wrapping_mul(0x1000)
                    .wrapping_sub((impulse_resp[k] as i32) * amp)
                    .wrapping_add(0x800)
                    >> 12) as i16;
            }
        } else {
            count_second_half += 1;
            let start = pos.saturating_sub(29);
            for k in start..60 {
                corr[k] = 0;
            }
            let rem = 60 - pos;
            for k in 0..rem {
                let t = target_sf[pos + k] as i32;
                target_sf[pos + k] = (t
                    .wrapping_mul(0x1000)
                    .wrapping_sub((impulse_resp[k] as i32) * amp)
                    .wrapping_add(0x800)
                    >> 12) as i16;
            }
        }

        // Matches the local_10 = 0 and (if local_10 <= iVar11) conditions in FUN_73b42dfe.
        let mut next_max: i32 = 0;
        if count_first_half < 3 {
            for i in 0..30 {
                if !used[i] {
                    if corr[i] == 0 {
                        let mut sum: i32 = 0;
                        for k in 0..30 {
                            sum = sum
                                .wrapping_add((impulse_resp[k] as i32) * (target_sf[i + k] as i32));
                        }
                        corr[i] = sum;
                    }
                    let a = wrapping_abs_i32(corr[i]);
                    if a >= next_max {
                        next_max = a;
                        best_pos = i;
                    }
                }
            }
        }

        if count_second_half < 4 {
            for i in 30..60 {
                if !used[i] {
                    if corr[i] == 0 {
                        let mut sum: i32 = 0;
                        let rem = 60 - i;
                        for k in 0..rem {
                            sum = sum
                                .wrapping_add((impulse_resp[k] as i32) * (target_sf[i + k] as i32));
                        }
                        corr[i] = sum;
                    }
                    let a = wrapping_abs_i32(corr[i]);
                    if a >= next_max {
                        next_max = a;
                        best_pos = i;
                    }
                }
            }
        }

        used[best_pos] = true;
        let mut sc: usize = if corr[best_pos] < 0 { 2 } else { 0 };
        if threshold <= (next_max >> 2) {
            sc += 1;
        }
        pulse_positions[p + 1] = best_pos;
        pulse_amps[p + 1] =
            (((TS_PULSE_SIGN_SCALE_53A[sc] as i32) * (base_gain as i32)) >> 15) as i16;
    }

    target_sf.fill(0);
    for p in 0..7 {
        target_sf[pulse_positions[p]] = pulse_amps[p];
    }

    // Encode pulse positions and signs with the combinatorial method from FUN_73b42dfe.
    let mut packed_signs: u16 = 0;
    let mut comb_high: u32 = 0;
    let mut table_idx = 30usize;
    let mut rem_pulses = 3i32;

    for i in 0..30 {
        let s = target_sf[i];
        if s == 0 {
            comb_high += TS_PULSE_COMBINADIC_140[table_idx] as u16 as u32;
        } else {
            packed_signs <<= 2;
            if s < 0 {
                packed_signs |= 2;
            }
            if (s as i32).abs() >= ((base_gain >> 1) as i32) {
                packed_signs |= 1;
            }
            rem_pulses -= 1;
            table_idx += 30;
        }
        table_idx += 1;
        if rem_pulses <= 0 {
            break;
        }
    }

    let mut comb_code = comb_high << 15;
    let mut table_idx2 = 0usize;
    let mut rem_pulses2 = 4i32;

    for i in 30..60 {
        let s = target_sf[i];
        if s == 0 {
            comb_code += TS_PULSE_COMBINADIC_140[table_idx2] as u16 as u32;
        } else {
            packed_signs <<= 2;
            if s < 0 {
                packed_signs |= 2;
            }
            if (s as i32).abs() >= ((base_gain >> 1) as i32) {
                packed_signs |= 1;
            }
            rem_pulses2 -= 1;
            table_idx2 += 30;
        }
        table_idx2 += 1;
        if rem_pulses2 <= 0 {
            break;
        }
    }

    params.pulse_signs_packed[subframe] = packed_signs;
    params.pulse_pos_codes[subframe] = comb_code;
}

/// Corresponds to `FUN_73b43bb2`: updates excitation history after selecting the adaptive vector and pulses.
fn update_excitation_history(
    excitation_sf: &mut [i16; 60],
    excitation_history: &mut [i16; 146],
    adaptive_vec: &[i16; 60],
) {
    excitation_history.copy_within(60..146, 0);
    for i in (0..60).rev() {
        let pulse = excitation_sf[i];
        let total = adaptive_vec[i].wrapping_add(pulse);
        excitation_sf[i] = total;
        excitation_history[86 + i] = total.wrapping_sub(pulse >> 3);
    }
}

/// Corresponds to `FUN_73b4279a`: updates weighted-synthesis history and encoder synthesis-filter state.
fn update_weighted_synthesis_history(
    weighted_history: &mut [i16; 146],
    lpc: &[i16; 8],
    excitation_sf: &[i16; 60],
    synth_state: &mut [i16; 8],
) {
    weighted_history.copy_within(60..146, 0);
    for n in 0..60 {
        let mut acc = (excitation_sf[n] as i32) << 12;
        for k in 0..8 {
            acc = acc.wrapping_add((lpc[k] as i32) * (synth_state[k] as i32));
        }
        weighted_history[86 + n] = (acc.wrapping_add(0x800) >> 12) as i16;
        for k in (1..8).rev() {
            synth_state[k] = synth_state[k - 1];
        }
        synth_state[0] = (acc >> 12) as i16;
    }
}

/// Packs the encoder parameters into one 32-byte TrueSpeech frame.
pub fn pack_bitstream(params: &FrameParams, out: &mut [u8; 32]) {
    let mut words = [0u32; 8];

    words[0] = (params.interp_flag as u32 & 0x01)
        | (((params.lpc_indices[0] as u32) & 0x1F) << 1)
        | (((params.lpc_indices[1] as u32) & 0x1F) << 6)
        | (((params.lpc_indices[2] as u32) & 0x0F) << 11)
        | (((params.lpc_indices[3] as u32) & 0x0F) << 15)
        | (((params.lpc_indices[4] as u32) & 0x0F) << 19)
        | (((params.lpc_indices[5] as u32) & 0x07) << 23)
        | (((params.lpc_indices[6] as u32) & 0x07) << 26)
        | (((params.lpc_indices[7] as u32) & 0x07) << 29);

    let lag0 = (params.base_pitch_lags[0] as u32) & 0xFF;
    let lag1 = (params.base_pitch_lags[1] as u32) & 0xFF;

    words[1] = ((params.adaptive_indices[0] as u32) & 0x7F)
        | (((params.adaptive_indices[1] as u32) & 0x7F) << 7)
        | (((params.adaptive_indices[2] as u32) & 0x7F) << 14)
        | (((params.adaptive_indices[3] as u32) & 0x7F) << 21)
        | (((lag0 >> 4) & 0x0F) << 28);

    words[2] = (params.pulse_signs_packed[0] as u32 & 0x3FFF)
        | (((params.pulse_signs_packed[1] as u32) & 0x3FFF) << 14)
        | ((lag1 & 0x0F) << 28);

    words[3] = (params.pulse_signs_packed[2] as u32 & 0x3FFF)
        | (((params.pulse_signs_packed[3] as u32) & 0x3FFF) << 14)
        | (((lag1 >> 4) & 0x0F) << 28);

    for sf in 0..4 {
        let gain = (params.pulse_gain_indices[sf] as u32) & 0x0F;
        let pos = params.pulse_pos_codes[sf] & 0x07FF_FFFF;
        let lag_bit = (lag0 >> sf) & 0x01;
        words[4 + sf] = gain | (pos << 4) | (lag_bit << 31);
    }

    for (i, &w) in words.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&w.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returned_packets_match_buffer_api_across_frames() {
        let mut returned = TrueSpeechContext::new();
        let mut buffered = TrueSpeechContext::new();
        for frame in 0..3 {
            let samples =
                std::array::from_fn(|i| ((i as i32 * 47 + frame * 311) % 8192 - 4096) as i16);
            let mut expected = [0; TRUESPEECH_PACKET_BYTES];
            buffered.encode_frame(&samples, &mut expected);
            assert_eq!(returned.encode_frame_to_packet(&samples), expected);
        }
    }

    #[test]
    fn version_and_format_initialization_match_the_codec_contract() {
        assert_eq!(TrueSpeechContext::version(), 0x0100_0000);
        let pcm_format: [i16; 11] = [1, 1, 8000, 0, 16000, 0, 2, 16, 0, 0, 0];
        let true_speech_format: [i16; 11] = [0x22, 1, 8000, 0, 1067, 0, 32, 1, 14, 1, 240];
        assert!(
            TrueSpeechContext::init_from_waveformat(&pcm_format, &true_speech_format).is_some()
        );
        assert!(
            TrueSpeechContext::init_from_waveformat(&true_speech_format, &pcm_format).is_some()
        );
    }

    #[test]
    fn encoder_produces_a_packet_for_speech_frames() {
        let mut encoder = TrueSpeechContext::new();
        let mut packet = [0u8; TRUESPEECH_PACKET_BYTES];
        for frame_index in 0..4 {
            let mut samples = [0i16; TRUESPEECH_FRAME_SAMPLES];
            for (index, sample) in samples.iter_mut().enumerate() {
                let time = (frame_index * TRUESPEECH_FRAME_SAMPLES + index) as f64
                    / TRUESPEECH_SAMPLE_RATE as f64;
                let value = 4000.0 * (2.0 * core::f64::consts::PI * 180.0 * time).sin()
                    + 2000.0 * (2.0 * core::f64::consts::PI * 540.0 * time).cos();
                *sample = value as i16;
            }
            encoder.encode_frame(&samples, &mut packet);
        }
        assert!(packet.iter().any(|byte| *byte != 0));
    }

    #[test]
    fn four_frame_packet_matches_original_dll_golden_vector() {
        // Generated by tsd32.dll from the deterministic integer pattern below.
        const EXPECTED_PACKETS: [u8; 128] = [
            0x41, 0xF3, 0xEB, 0x9D, 0xFF, 0x08, 0x70, 0x01, 0x88, 0xC8, 0xFF, 0x2F, 0x18, 0xF6,
            0xFA, 0x07, 0xC0, 0xB0, 0xDE, 0x7E, 0xAF, 0x8C, 0x28, 0x89, 0xDD, 0xFD, 0x09, 0x09,
            0x4A, 0xF3, 0x00, 0x41, 0x41, 0x73, 0xE4, 0x75, 0x62, 0x42, 0xC3, 0x05, 0xE0, 0x37,
            0x05, 0x22, 0x50, 0x04, 0x21, 0x01, 0x17, 0xA1, 0x40, 0x42, 0x69, 0xDD, 0x3A, 0xFE,
            0x1B, 0x30, 0xC5, 0x45, 0x8A, 0xFE, 0x49, 0x22, 0x01, 0x52, 0x4A, 0x75, 0x94, 0xD4,
            0xDF, 0x01, 0xAA, 0xAE, 0xA8, 0x2F, 0xA2, 0xFE, 0xF8, 0x0D, 0xAA, 0xA9, 0x2B, 0x2A,
            0x2A, 0x3F, 0xFD, 0xCC, 0xEF, 0xCB, 0x00, 0x00, 0xDB, 0x26, 0x82, 0x03, 0x41, 0xEB,
            0xDB, 0x95, 0x91, 0x01, 0xA1, 0x01, 0xC8, 0x3F, 0xA8, 0x2F, 0x04, 0x2A, 0x41, 0x00,
            0x49, 0x05, 0x59, 0x1E, 0xB9, 0x89, 0x69, 0x9E, 0x59, 0x48, 0xB4, 0x02, 0x3B, 0x39,
            0xA4, 0x3C,
        ];

        let mut input_samples = [0i16; 960];
        for (index, sample) in input_samples.iter_mut().enumerate() {
            *sample = (((index * 37 + 13) % 20_001) as i32 - 10_000) as i16;
        }

        let mut encoder = TrueSpeechContext::new();
        let mut actual_packets = [0u8; 128];
        assert_eq!(
            encoder.encode_samples_into(&input_samples, &mut actual_packets),
            Ok(EXPECTED_PACKETS.len())
        );
        assert_eq!(actual_packets, EXPECTED_PACKETS);
    }

    #[test]
    fn batch_encoder_zero_pads_only_the_last_partial_frame() {
        let input_samples = [321i16; TRUESPEECH_FRAME_SAMPLES + 1];
        let mut batch_encoder = TrueSpeechContext::new();
        let mut batch_packets = [0u8; 2 * TRUESPEECH_PACKET_BYTES];
        assert_eq!(
            TrueSpeechContext::encoded_packet_bytes(input_samples.len()),
            Some(batch_packets.len())
        );
        assert_eq!(
            batch_encoder.encode_samples_into(&input_samples, &mut batch_packets),
            Ok(batch_packets.len())
        );

        let mut frame_encoder = TrueSpeechContext::new();
        let mut first_frame = [0i16; TRUESPEECH_FRAME_SAMPLES];
        first_frame.copy_from_slice(&input_samples[..TRUESPEECH_FRAME_SAMPLES]);
        let mut expected_first_packet = [0u8; TRUESPEECH_PACKET_BYTES];
        frame_encoder.encode_frame(&first_frame, &mut expected_first_packet);

        let mut padded_last_frame = [0i16; TRUESPEECH_FRAME_SAMPLES];
        padded_last_frame[0] = input_samples[TRUESPEECH_FRAME_SAMPLES];
        let mut expected_last_packet = [0u8; TRUESPEECH_PACKET_BYTES];
        frame_encoder.encode_frame(&padded_last_frame, &mut expected_last_packet);

        assert_eq!(
            &batch_packets[..TRUESPEECH_PACKET_BYTES],
            &expected_first_packet
        );
        assert_eq!(
            &batch_packets[TRUESPEECH_PACKET_BYTES..],
            &expected_last_packet
        );
    }

    #[test]
    fn batch_encoder_rejects_short_output_before_changing_state() {
        let input_samples = [321i16; TRUESPEECH_FRAME_SAMPLES];
        let mut encoder = TrueSpeechContext::new();
        let mut short_output = [0xA5u8; TRUESPEECH_PACKET_BYTES - 1];
        assert_eq!(
            encoder.encode_samples_into(&input_samples, &mut short_output),
            Err(EncodeBufferError::OutputTooSmall {
                required: TRUESPEECH_PACKET_BYTES,
                provided: TRUESPEECH_PACKET_BYTES - 1,
            })
        );
        assert_eq!(short_output, [0xA5u8; TRUESPEECH_PACKET_BYTES - 1]);

        let mut actual_packet = [0u8; TRUESPEECH_PACKET_BYTES];
        let mut fresh_encoder = TrueSpeechContext::new();
        assert_eq!(
            encoder.encode_samples_into(&input_samples, &mut actual_packet),
            Ok(TRUESPEECH_PACKET_BYTES)
        );
        let mut expected_packet = [0u8; TRUESPEECH_PACKET_BYTES];
        fresh_encoder.encode_frame(&input_samples, &mut expected_packet);
        assert_eq!(actual_packet, expected_packet);
    }
}
