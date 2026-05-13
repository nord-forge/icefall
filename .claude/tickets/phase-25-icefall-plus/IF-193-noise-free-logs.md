# IF-193: Noise-Free Log Streams

**Phase:** 25 — Icefall+
**Priority:** Medium
**Estimate:** M

## Description

Smart log filtering that classifies log lines into signal vs noise using local pattern detection, highlights anomalies, and collapses repetitive entries. The log viewer shows a "smart" view by default with the option to see raw.

## Acceptance Criteria

- [ ] Log viewer toggle: "Smart" (default) / "Raw" view
- [ ] Smart view features:
  - **Collapse repetitive lines**: group identical or near-identical log lines with a count badge (e.g., "×47")
  - **Anomaly highlighting**: lines that are statistically unusual (new error patterns, first occurrence) get a yellow highlight
  - **Noise suppression**: common noise patterns auto-dimmed (health check pings, static asset requests, routine cron logs)
  - **Error clustering**: group related error lines together (stack trace + preceding context)
  - **Time gaps**: show a "5 minutes of silence" marker between activity bursts
- [ ] Pattern detection rules (local, no AI API):
  - Frequency-based: lines appearing 10+ times in 60 seconds are "repetitive"
  - Regex-based: known noise patterns (Caddy access logs for `/health`, favicon, robots.txt)
  - Level-based: ERROR/WARN always shown, DEBUG auto-hidden in smart mode
  - First-seen: any log line pattern not seen in the last 24 hours is "anomalous"
- [ ] Per-app customization: add custom noise patterns (regex) and custom highlight patterns
- [ ] "Show all" link on collapsed groups to expand
- [ ] SSE streaming: smart filtering applies in real-time on the incoming stream

## Technical Notes

- Pattern matching runs in the Rust backend (or agent for multi-server) before streaming via SSE
- Use a sliding window frequency counter (last 60 seconds) for repetition detection
- "First-seen" detection: Bloom filter of log line hashes from the last 24 hours
- No external AI calls — this is pure algorithmic filtering

## Dependencies

- IF-021 (Log viewer)
- IF-027 (Log storage and search)
