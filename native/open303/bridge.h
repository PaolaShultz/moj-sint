// Private C ABI. Project-authored, MIT. Never expose C++ ownership to Rust.
#pragma once
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
struct MojOpen303;
struct MojOpen303Controls {
    double waveform, cutoff_hz, resonance, env_mod, decay_ms;
    double accent, volume_db, tuning_hz, slide_ms;
};
struct MojOpen303Status { uint64_t clipped_samples; uint32_t faulted; };
struct MojOpen303* moj_open303_create(double rate, int mode, const struct MojOpen303Controls* controls);
void moj_open303_destroy(struct MojOpen303* voice);
int moj_open303_controls(struct MojOpen303* voice, const struct MojOpen303Controls* controls);
int moj_open303_note(struct MojOpen303* voice, int key, int velocity);
int moj_open303_retrigger_note(struct MojOpen303* voice, int key, int velocity);
int moj_open303_articulation(struct MojOpen303* voice, double normal_attack, double accent_attack, double accent_decay, double amp_decay);
void moj_open303_release_all(struct MojOpen303* voice);
void moj_open303_reset(struct MojOpen303* voice);
int moj_open303_idle(const struct MojOpen303* voice);
void moj_open303_render(struct MojOpen303* voice, float* output, size_t frames);
struct MojOpen303Status moj_open303_status(const struct MojOpen303* voice);
#ifdef __cplusplus
}
#endif
