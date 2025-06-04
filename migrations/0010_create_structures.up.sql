-- +migrate Up
CREATE TABLE structures (
  id                SERIAL PRIMARY KEY,
  owner_player_id   UUID REFERENCES players(id)   ON DELETE SET NULL,
  owner_faction_id  UUID REFERENCES factions(id)  ON DELETE SET NULL,
  type              TEXT NOT NULL,
  x                 INT  NOT NULL,
  y                 INT  NOT NULL,
  stats             JSONB NOT NULL DEFAULT '{}'::jsonb,
  placed_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX structures_tile_idx ON structures(x, y);
