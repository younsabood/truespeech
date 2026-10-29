#ifndef TRUESPEECH_H
#define TRUESPEECH_H

#include <stdint.h>

#if defined(_WIN32)
#  if defined(_M_IX86) || defined(__i386__)
#    define TRUESPEECH_CALL __stdcall
#  else
#    define TRUESPEECH_CALL
#  endif
#  if defined(TRUESPEECH_BUILD)
#    define TRUESPEECH_API __declspec(dllexport)
#  else
#    define TRUESPEECH_API __declspec(dllimport)
#  endif
#else
#  define TRUESPEECH_CALL
#  define TRUESPEECH_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* TrueSpeech format constants. */
#define TRUESPEECH_FORMAT_TAG UINT16_C(0x0022)
#define TRUESPEECH_SAMPLE_RATE UINT32_C(8000)
#define TRUESPEECH_SAMPLES_PER_FRAME UINT32_C(240)
#define TRUESPEECH_PACKET_BYTES UINT32_C(32)

/* Status values returned by stateful encoding functions. */
#define TRUESPEECH_STATUS_OK UINT32_C(0)
#define TRUESPEECH_STATUS_INVALID_ARGUMENT UINT32_C(1)

/*
 * Returns the codec version (currently 0x01000000).
 */
TRUESPEECH_API uint32_t TRUESPEECH_CALL TrueSpeech_Version(void);

/*
 * Creates an encoder state. Each format pointer must expose at least one
 * readable int16_t containing its WAVE format tag. The TrueSpeech descriptor
 * must expose at least eleven int16_t values; entries 9 and 10 must be 1 and
 * 240 respectively. Returns NULL for invalid tags or descriptor values.
 *
 * The encoder accepts mono signed 16-bit PCM at 8 kHz. The PCM descriptor's
 * remaining fields are the caller's responsibility and are not validated.
 * Release each returned state exactly once with TrueSpeech_Term.
 */
TRUESPEECH_API void *TRUESPEECH_CALL TrueSpeech_Init(
    const int16_t *input_format,
    const int16_t *output_format);

/*
 * Encodes exactly 240 PCM samples to exactly 32 packet bytes. `pcm` may be
 * unaligned. `packet` must address at least eight writable uint32_t words.
 * State, input, and output memory ranges must not overlap. Returns 0 on
 * success and 1 when any pointer is NULL. A state must not be used by
 * concurrent calls.
 */
TRUESPEECH_API uint32_t TRUESPEECH_CALL TrueSpeech_Encod(
    void *state,
    const int16_t *pcm,
    uint32_t *packet);

/* Clears a live state; returns 0 on success, 1 for a NULL state.
 * The state must not be used by concurrent calls. */
TRUESPEECH_API uint32_t TRUESPEECH_CALL TrueSpeech_Reset(void *state);

/* Releases a live state created by TrueSpeech_Init exactly once. NULL is
 * accepted. Do not use the state concurrently or after this call. */
TRUESPEECH_API uint32_t TRUESPEECH_CALL TrueSpeech_Term(void *state);

#ifdef __cplusplus
}
#endif

#endif /* TRUESPEECH_H */
