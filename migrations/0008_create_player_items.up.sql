-- +migrate Up
CREATE TABLE player_items (
  player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
  item_id   INT  NOT NULL REFERENCES items(id)   ON DELETE CASCADE,
  quantity  INT  NOT NULL CHECK (quantity > 0),
  PRIMARY KEY (player_id, item_id)
);
CREATE INDEX player_items_player_idx ON player_items(player_id);
