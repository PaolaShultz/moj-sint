// Project-authored regressions for the pinned Open303 core. MIT.
#include <cassert>
#include <climits>
#include <cstring>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <memory>
#include <new>
#include "rosic_Open303.h"
#include "../native/open303/bridge.cpp" // white-box fault injection, no production test API
#include <complex>
#include <vector>
#include <algorithm>
#include <limits>

namespace {
thread_local bool guarded = false;
thread_local unsigned allocations = 0, frees = 0;
struct Inspect : rosic::Open303 {
    double rate() const { return sampleRate; }
    double frequency() const { return oscFreq; }
    double activeAccent() const { return accentGain; }
    bool isIdle() const { return idle; }
};
}
void* operator new(std::size_t n) {
    if (guarded) ++allocations;
    if (void* p = std::malloc(n)) return p;
    throw std::bad_alloc();
}
void* operator new[](std::size_t n) { return ::operator new(n); }
void operator delete(void* p) noexcept { if (guarded && p) ++frees; std::free(p); }
void operator delete[](void* p) noexcept { ::operator delete(p); }
void operator delete(void* p, std::size_t) noexcept { ::operator delete(p); }
void operator delete[](void* p, std::size_t) noexcept { ::operator delete(p); }

void spectrum() {
    constexpr int n = 65536;
    for (int k : {55, 451, 3413}) for (int wave : {0, 1}) {
        auto core = std::make_unique<Inspect>();
        core->oscillator.setSampleRate(192000);
        core->oscillator.setBlendFactor(wave);
        core->oscillator.setFrequency(k * 48000.0 / n);
        core->oscillator.calculateIncrement();
        std::vector<std::complex<double>> data(n);
        for (int i = 0; i < 2*n; ++i) {
            double x = 0;
            for (int j = 0; j < 4; ++j) x = core->antiAliasFilter.getSample(-core->oscillator.getSample());
            if (i >= n) data[i-n] = x;
        }
        // Independent radix-2 FFT; do not use the inherited table-generation FFT.
        for (int i = 1, j = 0; i < n; ++i) {
            int bit = n >> 1;
            for (; j & bit; bit >>= 1) j ^= bit;
            j ^= bit;
            if (i < j) std::swap(data[i], data[j]);
        }
        for (int len = 2; len <= n; len *= 2) {
            auto base = std::polar(1.0, -2*PI/len);
            for (int i = 0; i < n; i += len) {
                std::complex<double> w(1,0);
                for (int j = 0; j < len/2; ++j) {
                    auto a = data[i+j], b = data[i+j+len/2]*w;
                    data[i+j] = a+b; data[i+j+len/2] = a-b; w *= base;
                }
            }
        }
        double signal = 0, other = 0;
        for (int i = 1; i <= n/2; ++i) {
            const double p = std::norm(data[i]) * (i == n/2 ? 1 : 2);
            if (i % k == 0) signal += p; else other += p;
        }
        const double db = 10*std::log10(other/signal);
        const double ceilings[] = {-63.5, -68.0, -89.5, -94.0, -113.5, -115.5};
        const int index = (k == 55 ? 0 : k == 451 ? 2 : 4) + wave;
        assert(std::isfinite(db) && db < ceilings[index]);
        std::printf("oscillator Hz=%.6f wave=%d nonharmonic_dBc=%.3f\n", k*48000.0/n, wave, db);
    }
}

int main(int argc, char** argv) {
    if (argc > 1 && std::strcmp(argv[1], "spectrum") == 0) { spectrum(); return 0; }
    auto s = std::make_unique<Inspect>();
    if (argc > 1 && std::strcmp(argv[1], "bounds") == 0) {
        // Exact upper bound must resolve to the last table, never index 12.
        assert(std::isfinite(s->waveTable1.getValueLinear(0, 0.0, 12)));
        return 0;
    }
    // The table builder uses flat double FFT buffers, with no Complex objects.
    rosic::FourierTransformerRadix2 fft;
    fft.setBlockSize(2048);
    double input[2048], packed[2048], restored[2048];
    for (int i = 0; i < 2048; ++i) input[i] = std::sin(i * 0.13) + 0.25;
    fft.transformRealSignal(input, packed);
    fft.transformSymmetricSpectrum(packed, restored);
    for (int i = 0; i < 2048; ++i) assert(std::abs(input[i] - restored[i]) < 1e-12);
    s->setSampleRate(48000);
    assert(s->rate() == 48000);
    assert(s->getWaveform() == 0.5);
    const double slide = s->pitchSlewLimiter.getTimeConstant();
    s->setSlideTime(s->getSlideTime());
    assert(s->pitchSlewLimiter.getTimeConstant() == slide);
    assert(s->getNormalAttack() == s->rc1.getTimeConstant());
    assert(s->getAccentAttack() == s->rc2.getTimeConstant());
    assert(EXPOFDBL(0.125) == -3);
    assert(EXPOFDBL(8.0) == 3);
    s->setTuning(432);
    s->setAccent(50);
    guarded = true;
    s->noteOn(69, 110, 0);
    for (int i = 0; i < 256; ++i) assert(std::isfinite(s->getSample()));
    s->noteOn(72, 70, 0);
    s->noteOn(72, 0, 0);
    assert(std::abs(s->frequency() - 432) < 1e-9);
    assert(s->activeAccent() == 0.5);
    s->setAccent(75); assert(s->activeAccent() == 0.75);
    s->setTuning(444); assert(std::abs(s->frequency()-444) < 1e-9);
    s->noteOn(72,70,0); s->setDecay(800);
    assert(s->mainEnv.getDecayTimeConstant() == 800);
    for (int i = 0; i < 1024; ++i) s->noteOn(i % 128, 110, 0);
    for (int i = 0; i < 128; ++i) s->noteOn(i, 0, 0);
    s->allNotesOff();
    guarded = false;
    assert(allocations == 0 && frees == 0);
    for (int i = 0; i < 480000; ++i) assert(std::isfinite(s->getSample()));
    assert(s->isIdle());
    // Prepare outside allocation guards, then test the actual C ABI as well.
    MojOpen303Controls controls{0.5, 700, 40, 35, 500, 60, -24, 440, 60};
    auto* bridge = moj_open303_create(48000, 0, &controls);
    assert(bridge);
    auto reference = std::make_unique<Inspect>();
    reference->setSampleRate(48000); reference->filter.setMode(rosic::TeeBeeFilter::TB_303);
    reference->setWaveform(controls.waveform); reference->setCutoff(controls.cutoff_hz);
    reference->setResonance(controls.resonance); reference->setEnvMod(controls.env_mod);
    reference->setDecay(controls.decay_ms); reference->setAccent(controls.accent);
    reference->setVolume(controls.volume_db); reference->setTuning(controls.tuning_hz);
    reference->setSlideTime(controls.slide_ms); reference->reset();
    allocations = frees = 0;
    guarded = true;
    assert(moj_open303_note(bridge, 36, 110)); reference->noteOn(36,110,0);
    float buffer[256];
    moj_open303_render(bridge,buffer,256);
    for (float x : buffer) assert(x == static_cast<float>(std::clamp(reference->getSample(),-.999,.999)));
    for (int i = 0; i < 1024; ++i) {
        assert(moj_open303_note(bridge, i%128, 110));
        assert(moj_open303_articulation(bridge, .3+(i%30), .3+(i%30), 30+(i%2970), 16+(i%2984)));
        assert(!moj_open303_articulation(bridge, NAN, 3, 200, 1230));
        assert(bridge->core.getNormalAttack() == .3+(i%30));
        assert(bridge->core.mainEnv.getDecayTimeConstant() == 30+(i%2970));
        controls.waveform = (i%128)/127.0;
        assert(moj_open303_controls(bridge,&controls));
        moj_open303_render(bridge,buffer,256);
        for (float x : buffer) assert(std::isfinite(x) && std::abs(x) <= 0.999f);
    }
    auto bad = controls; bad.cutoff_hz = std::numeric_limits<double>::quiet_NaN();
    assert(!moj_open303_controls(bridge,&bad));
    assert(!moj_open303_note(bridge,128,1));
    moj_open303_release_all(bridge);
    moj_open303_reset(bridge);
    moj_open303_render(bridge,buffer,256);
    for (float x : buffer) assert(x == 0);
    assert(moj_open303_idle(bridge));
    assert(!moj_open303_status(bridge).faulted);
    // Inject a poisoned internal state without exposing a production escape hatch.
    assert(moj_open303_note(bridge,36,110));
    bridge->core.ampEnv.setInternalState(std::numeric_limits<double>::quiet_NaN());
    moj_open303_render(bridge,buffer,256);
    assert(moj_open303_status(bridge).faulted);
    for (float x : buffer) assert(x == 0);
    moj_open303_reset(bridge);
    assert(moj_open303_note(bridge,36,110));
    moj_open303_render(bridge,buffer,256);
    assert(!moj_open303_status(bridge).faulted);
    for (float x : buffer) assert(std::isfinite(x));
    guarded = false;
    assert(allocations == 0 && frees == 0);
    moj_open303_destroy(bridge);
    std::puts("Open303 native bounds/state/C-ABI/allocation regressions passed");
}
