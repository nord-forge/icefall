# IF-259: Narrow tokio `full` to the features actually used

**Phase:** 31 — Efficiency
**Priority:** Low
**Estimate:** S

## Description

`tokio = { features = ["full"] }`. Verified usage in `src/` covers
`macros`, `rt-multi-thread`, `time`, `sync`, `signal`, `fs`, `process`, `net`,
`io-util` — i.e. **most** of `full`. So the win is real but **modest** (full also
pulls `test-util`, `parking_lot`, `io-uring` stubs, `stats`, etc.).

Worth doing for hygiene + a small size/build-time trim, but don't oversell it —
the audit's ~800 KB-1.2 MB estimate is likely high given how much of tokio is
actually used.

## Acceptance Criteria

- [ ] Replace `features = ["full"]` with the explicit minimal set that compiles
      and passes tests, e.g.:
      `["macros", "rt-multi-thread", "time", "sync", "signal", "fs", "process", "net", "io-util"]`
      (add any the compiler flags as missing).
- [ ] `cargo build` + full test suite pass.
- [ ] Binary size + clean-build time re-measured and recorded (set honest
      expectations — likely a few hundred KB, not multiple MB).
- [ ] `test-util` is NOT in the release feature set (it's test-only; if a test
      needs it, gate via dev-deps).

## Technical Notes

- The agent (the metrics collector, in icefall-agent) may have its own tokio
  feature set — scope this to the control-plane crate unless the agent shares it.

## Dependencies

- None.
