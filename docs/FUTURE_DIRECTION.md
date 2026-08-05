# Experimental direction

Status: open direction, not a scheduled implementation or production promise.

Moj Sint should grow from a small catalog of authored synthesis models into an
open experimental instrument laboratory. A musician should be able to play the
built-in models, inspect how they work, alter them, and eventually create a new
instrument with or without AI assistance.

“Open” means more than publishing source. It means providing a comprehensible
path from a sound idea to a playable model while Moj Sint continues to own the
difficult host, voice, timing, preset, and real-time safety boundaries. The
inside of a model may be strange; its boundary should remain predictable.

The most promising authoring surface is a low-code, typed micro-machine graph.
Small oscillators, counters, events, shapers, followers, resonators, filters,
delays, feedback elements, and stereo operations could be connected in a
strict text description. Moj Sint would validate types, cycles, resource
limits, deterministic state, and real-time suitability before rendering. This
is intended as a machine-native laboratory, not another unrestricted modular-
synth clone.

Every playable model should continue to expose eight meaningful timbral
controls plus ADSR. The internal graph can be complex, but the resulting
instrument must remain approachable through the same SHR-DAW control surface.
Factory models, community models, and unfinished experiments should remain
clearly distinguishable.

AI may help draft graphs, model code, control mappings, explanations, and test
ideas. AI authorship is not evidence of safety or musical value. Human- and
AI-authored models should pass the same deterministic, finite-output,
allocation, resource, alias/error, stereo, chord, cleanup, and native-target
checks. Human listening remains the decision about whether an experiment is
worth keeping.

A familiar swarm or supersaw-like sound is one possible learning experiment
for this architecture. It could make oscillator population, detuning, phase,
coupling, motion, stereo spread, normalization, and spectral shaping easy to
hear while leaving production effects to SHR-DAW. The goal would not be to
copy a workstation synth or promise a particular model, but to discover
whether the micro-machine format makes a recognisable target lighter, more
inspectable, and characteristically Moj Sint.

The detailed technical possibilities and safety boundaries remain in
[Typed Micro-Machine Routing](MICRO_MACHINE_ROUTING.md). Implementation should
start with a deliberately small experiment only after its exact scope and
listening question are chosen.
