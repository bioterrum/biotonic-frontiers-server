-- +migrate Up
CREATE TABLE chat_messages (
  id          SERIAL PRIMARY KEY,
  faction_id  UUID NOT NULL REFERENCES factions(id) ON DELETE CASCADE,
  sender_id   UUID NOT NULL REFERENCES players(id)  ON DELETE CASCADE,
  content     TEXT NOT NULL CHECK (length(content) <= 500),
  sent_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX chat_messages_faction_idx ON chat_messages(faction_id);
