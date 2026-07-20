#ifndef SALIERI_C_GAIN_H
#define SALIERI_C_GAIN_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SalieriCGainState {
    float gain;
    uint32_t sample_rate;
    uint16_t channels;
    size_t max_block_frames;
} SalieriCGainState;

enum {
    SALI_SUCCESS = 0,
    SALI_ERR_NULL = -1,
    SALI_ERR_INVALID_PARAMETER = -2,
    SALI_ERR_FRAME_OR_CHANNEL_MISMATCH = -3,
    SALI_ERR_NON_FINITE = -4
};

void salieri_c_gain_reset(SalieriCGainState *state);
int salieri_c_gain_prepare(
    SalieriCGainState *state,
    uint32_t sample_rate,
    uint16_t channels,
    size_t max_block_frames
);
int salieri_c_gain_set_gain(SalieriCGainState *state, float gain);
int salieri_c_gain_process(
    SalieriCGainState *state,
    float *interleaved,
    size_t frames,
    uint16_t channels
);

#ifdef __cplusplus
}
#endif

#endif
