# IF-258: `panic = "abort"` in the release profile

**Phase:** 31 — Efficiency
**Priority:** Low
**Estimate:** S

## Description

`[profile.release]` has `lto`, `codegen-units = 1`, `strip` — but not
`panic = "abort"`. Without it the binary carries full stack-unwinding tables
(~150-250 KB) for a panic path that, on a server, almost always ends in a
process restart anyway (systemd `Restart=on-failure`).

## Acceptance Criteria

- [ ] Add `panic = "abort"` to `[profile.release]` in Cargo.toml.
- [ ] Full test suite still passes (note: `#[should_panic]` tests and any
      `catch_unwind` won't work under abort — verify none are relied on in
      release-mode integration paths; unit tests run under the test profile,
      which is unaffected, so this is usually fine).
- [ ] Confirm the systemd unit restarts on failure (it does:
      `Restart=on-failure` in install.sh) so an abort is recovered cleanly.
- [ ] Binary size re-measured and recorded.

## Technical Notes

- `panic = "abort"` only affects the release profile here; `cargo test` uses the
  test profile and is unaffected, so `#[should_panic]` tests keep working.
- Do NOT also set `opt-level = "z"/"s"` in this ticket — that trades request
  latency for size and needs its own measurement. Keep this change to the
  unambiguous unwinding-table win.

## Dependencies

- None.
