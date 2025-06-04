-- +migrate Up
ALTER TABLE games
    ADD COLUMN winner_id        UUID REFERENCES players(id),
    ADD COLUMN player1_elo_delta INT  NOT NULL DEFAULT 0,
    ADD COLUMN player2_elo_delta INT  NOT NULL DEFAULT 0;
