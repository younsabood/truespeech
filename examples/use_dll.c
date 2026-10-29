#include "truespeech.h"

#include <stdio.h>

int main(void) {
    /* Only the TrueSpeech tag and descriptor words 9/10 are currently read. */
    const int16_t pcm_format[11] = {1, 1, 8000, 0, 16000, 0, 2, 16, 0, 0, 0};
    const int16_t truespeech_format[11] = {
        0x22, 1, 8000, 0, 1067, 0, 32, 1, 14, 1, 240
    };
    int16_t pcm[TRUESPEECH_SAMPLES_PER_FRAME];
    uint32_t packet_words[TRUESPEECH_PACKET_BYTES / sizeof(uint32_t)];
    void *state = TrueSpeech_Init(pcm_format, truespeech_format);
    if (state == NULL) {
        fputs("TrueSpeech_Init failed\n", stderr);
        return 1;
    }

    FILE *output = fopen("truespeech.packets", "wb");
    if (output == NULL) {
        (void)TrueSpeech_Term(state);
        perror("fopen");
        return 1;
    }

    for (size_t frame = 0; frame < 100; ++frame) {
        for (size_t sample = 0; sample < TRUESPEECH_SAMPLES_PER_FRAME; ++sample) {
            pcm[sample] = (int16_t)((int32_t)(((frame * TRUESPEECH_SAMPLES_PER_FRAME + sample) % 200) * 100) - 10000);
        }
        if (TrueSpeech_Encod(state, pcm, packet_words) != TRUESPEECH_STATUS_OK ||
            fwrite(packet_words, 1, TRUESPEECH_PACKET_BYTES, output) != TRUESPEECH_PACKET_BYTES) {
            fputs("Encoding or output write failed\n", stderr);
            fclose(output);
            (void)TrueSpeech_Term(state);
            return 1;
        }
    }

    if (fclose(output) != 0) {
        perror("fclose");
        (void)TrueSpeech_Term(state);
        return 1;
    }
    return (int)TrueSpeech_Term(state);
}
