-- +migrate Down
ALTER TABLE land_parcels
    DROP COLUMN IF EXISTS owner_player_id;
