-- Remove the unused tunnel_enabled flag from apps.
-- Cloudflare Tunnel integration (IF-151) and the Secure Tunnel Debugger (IF-190)
-- were never implemented; this drops the only schema remnant.
ALTER TABLE apps DROP COLUMN tunnel_enabled;
