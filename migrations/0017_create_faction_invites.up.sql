-- +migrate Up
CREATE TABLE faction_invites (
  id               UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  faction_id       UUID NOT NULL REFERENCES factions(id) ON DELETE CASCADE,
  invited_player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
  invited_by       UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at       TIMESTAMPTZ NOT NULL,
  UNIQUE (faction_id, invited_player_id)
);
