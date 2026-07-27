#ifndef TRK_C_GAIN_H
#define TRK_C_GAIN_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct TrkCGainState {
    float gain;
    uint32_t sample_rate;
    uint16_t channels;
    size_t max_block_frames;
} TrkCGainState;

enum {
    TRK_SUCCESS = 0,
    TRK_ERR_NULL = -1,
    TRK_ERR_INVALID_PARAMETER = -2,
    TRK_ERR_FRAME_OR_CHANNEL_MISMATCH = -3,
    TRK_ERR_NON_FINITE = -4
};

void trk_c_gain_reset(TrkCGainState *state);
int trk_c_gain_prepare(
    TrkCGainState *state,
    uint32_t sample_rate,
    uint16_t channels,
    size_t max_block_frames
);
int trk_c_gain_set_gain(TrkCGainState *state, float gain);
int trk_c_gain_process(
    TrkCGainState *state,
    float *interleaved,
    size_t frames,
    uint16_t channels
);

#ifdef __cplusplus
}
#endif

#endif
