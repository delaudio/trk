#include "trk_c_gain.h"

#include <math.h>

void trk_c_gain_reset(TrkCGainState *state) {
    if (state == NULL) {
        return;
    }
    state->gain = 1.0f;
    state->sample_rate = 0;
    state->channels = 0;
    state->max_block_frames = 0;
}

int trk_c_gain_prepare(
    TrkCGainState *state,
    uint32_t sample_rate,
    uint16_t channels,
    size_t max_block_frames
) {
    if (state == NULL) {
        return TRK_ERR_NULL;
    }
    if (sample_rate == 0 || channels == 0 || max_block_frames == 0) {
        return TRK_ERR_INVALID_PARAMETER;
    }
    state->sample_rate = sample_rate;
    state->channels = channels;
    state->max_block_frames = max_block_frames;
    return TRK_SUCCESS;
}

int trk_c_gain_set_gain(TrkCGainState *state, float gain) {
    if (state == NULL) {
        return TRK_ERR_NULL;
    }
    if (!isfinite(gain) || gain < 0.0f || gain > 2.0f) {
        return TRK_ERR_INVALID_PARAMETER;
    }
    state->gain = gain;
    return TRK_SUCCESS;
}

int trk_c_gain_process(
    TrkCGainState *state,
    float *interleaved,
    size_t frames,
    uint16_t channels
) {
    if (state == NULL || interleaved == NULL) {
        return TRK_ERR_NULL;
    }
    if (
        state->channels == 0 ||
        channels != state->channels ||
        frames > state->max_block_frames
    ) {
        return TRK_ERR_FRAME_OR_CHANNEL_MISMATCH;
    }
    const size_t samples = frames * (size_t)channels;
    for (size_t index = 0; index < samples; index++) {
        if (!isfinite(interleaved[index])) {
            return TRK_ERR_NON_FINITE;
        }
        interleaved[index] *= state->gain;
        if (!isfinite(interleaved[index])) {
            return TRK_ERR_NON_FINITE;
        }
    }
    return TRK_SUCCESS;
}
