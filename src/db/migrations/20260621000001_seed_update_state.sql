-- The update_state table is a singleton (id = 1) but the baseline schema never
-- seeded the row, so every `... WHERE id = 1` read/write hit zero rows:
-- get/set update preferences returned 500, and the auto-update scheduler logged
-- "no rows returned" on every check. Seed the row idempotently.
--
-- highest_seen_version = '0.0.0' so the first real release is detected as newer.
INSERT OR IGNORE INTO update_state (id, highest_seen_version) VALUES (1, '0.0.0');
