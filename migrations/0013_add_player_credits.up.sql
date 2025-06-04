-- +migrate Up
ALTER TABLE players
    ADD COLUMN credits BIGINT NOT NULL DEFAULT 0;
