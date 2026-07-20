#include "salieri_c_gain.h"

#include <math.h>

void salieri_c_gain_reset(SalieriCGainState *state) {
    if (state == NULL) {
        return;
    }
    state->gain = 1.0f;
    state->sample_rate = 0;
    state->channels = 0;
    state->max_block_frames = 0;
}

int salieri_c_gain_prepare(
    SalieriCGainState *state,
    uint32_t sample_rate,
    uint16_t channels,
    size_t max_block_frames
) {
    if (state == NULL) {
        return SALI_ERR_NULL;
    }
    if (sample_rate == 0 || channels == 0 || max_block_frames == 0) {
        return SALI_ERR_INVALID_PARAMETER;
    }
    state->sample_rate = sample_rate;
    state->channels = channels;
    state->max_block_frames = max_block_frames;
    return SALI_SUCCESS;
}

int salieri_c_gain_set_gain(SalieriCGainState *state, float gain) {
    if (state == NULL) {
        return SALI_ERR_NULL;
    }
    if (!isfinite(gain) || gain < 0.0f || gain > 2.0f) {
        return SALI_ERR_INVALID_PARAMETER;
    }
    state->gain = gain;
    return SALI_SUCCESS;
}

int salieri_c_gain_process(
    SalieriCGainState *state,
    float *interleaved,
    size_t frames,
    uint16_t channels
) {
    if (state == NULL || interleaved == NULL) {
        return SALI_ERR_NULL;
    }
    if (
        state->channels == 0 ||
        channels != state->channels ||
        frames > state->max_block_frames
    ) {
        return SALI_ERR_FRAME_OR_CHANNEL_MISMATCH;
    }
    const size_t samples = frames * (size_t)channels;
    for (size_t index = 0; index < samples; index++) {
        if (!isfinite(interleaved[index])) {
            return SALI_ERR_NON_FINITE;
        }
        interleaved[index] *= state->gain;
        if (!isfinite(interleaved[index])) {
            return SALI_ERR_NON_FINITE;
        }
    }
    return SALI_SUCCESS;
}
