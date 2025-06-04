-- +migrate Up
CREATE TABLE items (
  id          SERIAL PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  description TEXT,
  base_price  INT  NOT NULL
);
