# IF-260: Server metrics collection — drop the 200 ms blocking sleep

**Phase:** 31 — Efficiency
**Priority:** Low
**Estimate:** M

## Description

The control-plane server-metrics collector (`src/api/routes/server.rs`, the
`spawn_blocking` loop around line 96) re-creates a `sysinfo::System` every cycle
and does a **double `refresh_cpu_all()` with a 200 ms blocking `std::thread::sleep`
between them** to get a CPU-usage delta. It runs every 2 s, so a blocking-pool
thread is parked ~200 ms out of every 2 s (~10% duty cycle) purely to settle the
CPU measurement, plus a full `/proc` re-parse from `System::new()` each time.

sysinfo needs *two samples over an interval* to compute CPU %, but those two
samples can be **this cycle's and the previous cycle's** — a persistent `System`
refreshed once per 2 s loop gives the delta for free, with no in-cycle sleep and
no re-init.

## Acceptance Criteria

- [ ] Hold a persistent `sysinfo::System` (and `Disks`) across collection cycles
      instead of `System::new()` each time.
- [ ] Remove the in-cycle 200 ms `std::thread::sleep` + the second
      `refresh_cpu_all()`; rely on the inter-cycle interval (2 s) for the CPU
      delta. First cycle may report 0% CPU (acceptable — document it).
- [ ] Refresh disks at a coarser cadence (e.g. every ~30-60 s), not every cycle
      — disk topology rarely changes; also covers `Disks::new_with_refreshed_list()`
      in `src/update/download.rs` if cheap to share.
- [ ] CPU/memory numbers remain accurate (spot-check against `top`/`free`).
- [ ] The blocking task no longer parks a thread for 200 ms each cycle.

## Technical Notes

- Keep using `spawn_blocking` (sysinfo is sync), but make the `System` live in
  the task's loop scope so it persists across iterations.
- Watch the first-sample edge case: sysinfo's first `refresh_cpu_all()` after
  construction has no prior sample, so CPU% is 0 until the second refresh — with
  a persistent System that just means the very first reading is 0.

## Dependencies

- None. Runtime-only (no binary-size impact).
