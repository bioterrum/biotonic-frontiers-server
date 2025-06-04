-- +migrate Up
CREATE TABLE players (
  id          UUID PRIMARY KEY     DEFAULT uuid_generate_v4(),
  user_id     UUID NOT NULL        REFERENCES users(id) ON DELETE CASCADE,
  nickname    TEXT NOT NULL UNIQUE,
  elo_rating  INT  NOT NULL        DEFAULT 1500,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX players_user_id_idx ON players(user_id);
