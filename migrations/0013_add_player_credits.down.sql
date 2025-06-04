-- +migrate Down
ALTER TABLE players
    DROP COLUMN IF EXISTS credits;
