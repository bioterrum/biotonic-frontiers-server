-- +migrate Up
CREATE TABLE chain_events (
  version  BIGINT PRIMARY KEY,
  hash     TEXT NOT NULL,
  payload  JSONB NOT NULL,
  seen_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);