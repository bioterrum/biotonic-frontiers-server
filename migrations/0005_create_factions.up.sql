-- +migrate Up
CREATE TABLE factions (
  id          UUID PRIMARY KEY     DEFAULT uuid_generate_v4(),
  name        TEXT NOT NULL UNIQUE,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
