-- +migrate Up
CREATE TABLE games (
  id           UUID PRIMARY KEY     DEFAULT uuid_generate_v4(),
  player1_id   UUID NOT NULL        REFERENCES users(id) ON DELETE CASCADE,
  player2_id   UUID                 REFERENCES users(id) ON DELETE SET NULL,
  state        TEXT NOT NULL,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- keep updated_at current on every row update
CREATE OR REPLACE FUNCTION trigger_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_updated_at
  BEFORE UPDATE ON games
  FOR EACH ROW
  EXECUTE PROCEDURE trigger_set_updated_at();
