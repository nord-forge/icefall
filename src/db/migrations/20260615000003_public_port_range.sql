-- IF-172: Public port / TCP proxy for database access
--
-- The public_ports table and its allocation helpers already exist in the
-- baseline schema. This migration adds the admin-configurable port range to the
-- global proxy_settings singleton so the allocator knows which ports it may hand
-- out. Defaults to 10000-10100 (100 ports), matching the ticket.

ALTER TABLE proxy_settings ADD COLUMN public_port_range_start INTEGER NOT NULL DEFAULT 10000;
ALTER TABLE proxy_settings ADD COLUMN public_port_range_end INTEGER NOT NULL DEFAULT 10100;
