-- Phase 25 batch 1: S-sized parity gap tickets

-- IF-159: Registration toggle (already exists in registration_settings table, just ensure default)
-- registration_settings table already has allow_registration field from previous migration

-- IF-160: Monorepo base directory
ALTER TABLE apps ADD COLUMN base_directory TEXT;

-- IF-161: Multiple domains — primary indicator
ALTER TABLE domains ADD COLUMN is_primary BOOLEAN NOT NULL DEFAULT FALSE;

-- IF-162: Deploy by tag
ALTER TABLE deploys ADD COLUMN tag TEXT;

-- IF-164: Configurable backup retention
ALTER TABLE databases ADD COLUMN backup_retention_count INTEGER NOT NULL DEFAULT 7;

-- IF-171: Internal URL generation — hostname set at container creation, no schema change needed

-- IF-173: Raw Compose mode — uses existing deploy_mode field with value 'raw-compose'
