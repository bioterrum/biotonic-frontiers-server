-- +migrate Up
ALTER TABLE games
    DROP CONSTRAINT IF EXISTS games_player1_id_fkey,
    DROP CONSTRAINT IF EXISTS games_player2_id_fkey;

ALTER TABLE games
    ADD CONSTRAINT games_player1_id_fkey
        FOREIGN KEY (player1_id)
        REFERENCES players(id) ON DELETE CASCADE,
    ADD CONSTRAINT games_player2_id_fkey
        FOREIGN KEY (player2_id)
        REFERENCES players(id) ON DELETE SET NULL;
