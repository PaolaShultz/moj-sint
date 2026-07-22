# Repository Instructions

- Moj Sint is a fourth external SHR-DAW instrument. Never disguise its presets,
  controls, or process as synthv1.
- Keep `Engine` and DSP independent of JACK, ALSA, files, processes, and clocks.
- Use test-first development. Every render-path change needs finite-output and
  allocation checks; oscillator/nonlinear changes need aliasing measurements.
- The live callback may not allocate/deallocate, lock, perform I/O, log, format,
  panic, spawn, or perform avoidable per-sample transcendental setup.
- Preserve the thirteen stable musical roles in `docs/HANDOFF.md`. Do not claim
  a macro is useful until automated sweeps and human listening both pass.
- Treat scalar Rust as the baseline. Add x86_64/AArch64 specialization only
  behind measured compile-time gates.
- Run `cargo fmt --check`, all-target tests, Clippy with warnings denied, release
  build, audit/deny checks, and deterministic render comparison before handoff.
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
