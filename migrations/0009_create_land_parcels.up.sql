-- +migrate Up
CREATE TABLE land_parcels (
  id                SERIAL PRIMARY KEY,
  biome_type        TEXT NOT NULL,
  owner_faction_id  UUID REFERENCES factions(id) ON DELETE SET NULL,
  x                 INT  NOT NULL,
  y                 INT  NOT NULL,
  created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (x, y)
);
