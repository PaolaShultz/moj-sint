# Repository Instructions

- For planning, architecture, listening decisions, handoffs, or ambiguous
  context, consult the `Moj Sint` index in
  `/home/shome/Documents/knowledge` with `zk`, then verify against
  `docs/HANDOFF.md`, repository source/tests, and live Git state.
- `docs/HANDOFF.md` remains the durable project source of truth. After material
  state, workflow, or decision changes, update it first, then update the
  relevant concise knowledge note and run
  `/home/shome/Documents/knowledge/.zk/validate.sh`.
- Agents own knowledge organization and synchronization; the user is not
  expected to edit notes. Never store exact HEAD, clean/ahead status, or
  disposable artifact existence as current knowledge.
- Moj Sint is a fourth external SHR-DAW instrument. Never disguise its presets,
  controls, or process as synthv1.
- Keep `Engine` and DSP independent of JACK, ALSA, files, processes, and clocks.
- Use test-first development. Every render-path change needs finite-output and
  allocation checks; oscillator/nonlinear changes need aliasing measurements.
- The live callback may not allocate/deallocate, lock, perform I/O, log, format,
  panic, spawn, or perform avoidable per-sample transcendental setup.
- Preserve the model-specific physical surface in `docs/PRESET_SCHEMA.md`:
  the first five models use seven timbre controls, volume at position 5, and
  ADSR; Pressure Chain uses eight timbre controls plus amp ADSR; Dual Filter
  uses fifteen values and its dedicated core click. SHR owns AUX sends at
  positions 13–15 for the twelve-position models. Do not invent another
  button, mode, or master-encoder takeover.
- Follow SHR-DAW's actual live-control path. Permission to apply a value on the
  next note is an implementation option, not a product constraint; defer a
  parameter only when its safe implementation genuinely requires it.
- Do not assume eight-voice polyphony. Measure 1/2/4/8 voices on the native Pi;
  four is an acceptable outcome if callback/headroom and listening evidence
  support it.
- Treat scalar Rust as the baseline. Add x86_64/AArch64 specialization only
  behind measured compile-time gates.
- Run `cargo fmt --check`, the normal all-target test suite, Clippy with warnings
  denied, release build, audit/deny checks, and the production deterministic
  render comparison before a release handoff. Historical research/audition
  renderers, exhaustive evidence matrices, and native benchmark smoke runs are
  development-only ignored tests; run them with `cargo test --all-targets
  --all-features -- --ignored` only when their own code, evidence, or protected
  assumption changes, or when the user explicitly requests them. The agent
  owns this classification and test selection.
- Do not start/restart JACK, connect hardware, modify SHR-DAW, or make Pi
  performance/latency/polyphony/sound-quality claims without explicit scope and
  native evidence.
- Preserve primary-source authorship, publication, URLs, and licensing notes in
  research documentation. Do not copy third-party DSP source, presets, samples,
  prose, or figures without a separate license review.
- Experimental outputs are disposable by default. Keep them under the ignored
  `artifacts/` tree or outside the repository, document rejected conclusions,
  and delete the generated batch instead of archiving it.
- Never preserve an experimental batch, tone, report, parameter/preset file, or
  other experiment output in Git automatically. You may offer to copy a
  specific worthwhile result into a separate tracked directory, but do so only
  after the user explicitly requests that preservation. This rule applies to
  experiment outputs, not ordinary source, tests, or durable project docs.
- Listening variations must test fundamentally different source mechanisms or
  topologies. Parameter positions within one graph do not count as variations;
  do not present batches of near-identical saws, sines, or processed versions
  of one tone. Parameter sweeps remain valid only as automated engineering
  checks.
