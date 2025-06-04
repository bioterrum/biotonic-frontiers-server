-- +migrate Up
CREATE TABLE faction_members (
  faction_id  UUID NOT NULL REFERENCES factions(id) ON DELETE CASCADE,
  player_id   UUID NOT NULL REFERENCES players(id)  ON DELETE CASCADE,
  role        TEXT NOT NULL DEFAULT 'member',
  joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (faction_id, player_id)
);
CREATE INDEX faction_members_player_idx ON faction_members(player_id);
