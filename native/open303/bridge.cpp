// Project-authored boundary for the separately licensed Open303 DSP.
#include "bridge.h"
#include "rosic_Open303.h"
#include <algorithm>
#include <cmath>
#include <limits>
#include <memory>

struct MojOpen303 {
    rosic::Open303 core;
    MojOpen303Controls controls{};
    MojOpen303Status status{};
    double articulation[4] = {3.0, 3.0, 200.0, 1230.0};
};
namespace {
bool in_range(double value, double lo, double hi) {
    return std::isfinite(value) && value >= lo && value <= hi;
}
bool valid(const MojOpen303Controls& c) {
    return in_range(c.waveform, 0, 1) && in_range(c.cutoff_hz, 314, 2394)
        && in_range(c.resonance, 0, 100) && in_range(c.env_mod, 0, 100)
        && in_range(c.decay_ms, 200, 2000) && in_range(c.accent, 0, 100)
        && in_range(c.volume_db, -60, 0) && in_range(c.tuning_hz, 400, 480)
        && in_range(c.slide_ms, 0, 300);
}
void apply(MojOpen303& v, const MojOpen303Controls& c, bool force) {
    // Event-time setup only. Never rebuild waveform tables here.
    auto& old = v.controls;
    if (force || c.waveform != old.waveform) v.core.setWaveform(c.waveform);
    if (force || c.cutoff_hz != old.cutoff_hz) v.core.setCutoff(c.cutoff_hz);
    if (force || c.resonance != old.resonance) v.core.setResonance(c.resonance);
    if (force || c.env_mod != old.env_mod) v.core.setEnvMod(c.env_mod);
    if (force || c.decay_ms != old.decay_ms) v.core.setDecay(c.decay_ms);
    if (force || c.accent != old.accent) v.core.setAccent(c.accent);
    if (force || c.volume_db != old.volume_db) v.core.setVolume(c.volume_db);
    if (force || c.tuning_hz != old.tuning_hz) v.core.setTuning(c.tuning_hz);
    if (force || c.slide_ms != old.slide_ms) v.core.setSlideTime(c.slide_ms);
    old = c;
}
}
extern "C" {
MojOpen303* moj_open303_create(double rate, int mode, const MojOpen303Controls* c) {
    if (!c || !valid(*c) || !in_range(rate, 44100, 96000) || (mode != 0 && mode != 1)) return nullptr;
    try {
        auto v = std::make_unique<MojOpen303>();
        v->core.setSampleRate(rate);
        v->core.filter.setMode(mode == 0 ? rosic::TeeBeeFilter::TB_303 : rosic::TeeBeeFilter::LP_18);
        apply(*v, *c, true);
        v->core.reset();
        return v.release();
    } catch (...) { return nullptr; } // Creation is outside the audio thread.
}
void moj_open303_destroy(MojOpen303* v) { delete v; }
int moj_open303_controls(MojOpen303* v, const MojOpen303Controls* c) {
    if (!v || !c || !valid(*c)) return 0;
    apply(*v, *c, false);
    return 1;
}
int moj_open303_note(MojOpen303* v, int key, int velocity) {
    if (!v || key < 0 || key > 127 || velocity < 0 || velocity > 127) return 0;
    if (!v->status.faulted) v->core.noteOn(key, velocity, 0);
    return 1;
}
int moj_open303_retrigger_note(MojOpen303* v, int key, int velocity) {
    if (!v || key < 0 || key > 127 || velocity < 0 || velocity > 127) return 0;
    if (!v->status.faulted) v->core.noteOn(key, velocity, 0, true);
    return 1;
}
int moj_open303_articulation(MojOpen303* v, double normal, double attack, double decay, double amp) {
    if (!v || !in_range(normal, .3, 30) || !in_range(attack, .3, 30)
        || !in_range(decay, 30, 3000) || !in_range(amp, 16, 3000)) return 0;
    auto& old = v->articulation;
    if (old[0] != normal) v->core.setNormalAttack(normal);
    if (old[1] != attack) v->core.setAccentAttack(attack);
    if (old[2] != decay) v->core.setAccentDecay(decay);
    if (old[3] != amp) v->core.setAmpDecay(amp);
    old[0] = normal; old[1] = attack; old[2] = decay; old[3] = amp;
    return 1;
}
int moj_open303_pitch_bend(MojOpen303* v, double semitones) {
    if (!v || !in_range(semitones, -2.5, 2.5)) return 0;
    v->core.setPitchBend(semitones);
    return 1;
}
void moj_open303_release_all(MojOpen303* v) { v->core.allNotesOff(); }
void moj_open303_reset(MojOpen303* v) { v->core.reset(); v->status = {}; }
int moj_open303_idle(const MojOpen303* v) { return v->core.isIdle(); }
void moj_open303_render(MojOpen303* v, float* out, size_t frames) {
    for (size_t i = 0; i < frames; ++i) {
        if (v->status.faulted) { out[i] = 0; continue; }
        const double sample = v->core.getSample();
        if (!std::isfinite(sample)) {
            v->core.reset(); v->status.faulted = 1; out[i] = 0;
        } else {
            // Explicit presentation ceiling, outside inherited DSP. No normalization.
            if (std::abs(sample) > 0.999 && v->status.clipped_samples != std::numeric_limits<uint64_t>::max())
                ++v->status.clipped_samples;
            out[i] = static_cast<float>(std::clamp(sample, -0.999, 0.999));
        }
    }
}
MojOpen303Status moj_open303_status(const MojOpen303* v) { return v->status; }
}
