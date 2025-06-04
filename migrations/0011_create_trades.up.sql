-- +migrate Up
CREATE TABLE trades (
  id           SERIAL PRIMARY KEY,
  from_player  UUID NOT NULL REFERENCES players(id),
  to_player    UUID NOT NULL REFERENCES players(id),
  item_id      INT  NOT NULL REFERENCES items(id),
  qty          INT  NOT NULL CHECK (qty > 0),
  price        INT  NOT NULL CHECK (price >= 0),
  executed_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX trades_from_player_idx ON trades(from_player);
CREATE INDEX trades_to_player_idx   ON trades(to_player);
