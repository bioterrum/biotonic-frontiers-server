-- +migrate Down
ALTER TABLE factions
    DROP COLUMN IF EXISTS description,
    DROP COLUMN IF EXISTS logo_url;
