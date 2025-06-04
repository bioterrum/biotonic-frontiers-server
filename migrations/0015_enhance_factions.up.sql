-- +migrate Up
ALTER TABLE factions
    ADD COLUMN description TEXT NOT NULL DEFAULT '',
    ADD COLUMN logo_url    TEXT;
