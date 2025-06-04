-- +migrate Up
ALTER TABLE land_parcels
    ADD COLUMN owner_player_id UUID
        REFERENCES players(id) ON DELETE SET NULL;

CREATE INDEX land_parcels_owner_player_idx
    ON land_parcels(owner_player_id);
