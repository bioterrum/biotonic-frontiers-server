-- +migrate Down
DROP TRIGGER IF EXISTS set_updated_at ON games;
DROP FUNCTION IF EXISTS trigger_set_updated_at;
DROP TABLE games;
