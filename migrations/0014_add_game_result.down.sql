-- +migrate Down
ALTER TABLE games
    DROP COLUMN IF EXISTS winner_id,
    DROP COLUMN IF EXISTS player1_elo_delta,
    DROP COLUMN IF EXISTS player2_elo_delta;
