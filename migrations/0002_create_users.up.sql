-- +migrate Up
CREATE TABLE users (
  id         UUID PRIMARY KEY      DEFAULT uuid_generate_v4(),
  email      TEXT NOT NULL UNIQUE,
  created_at TIMESTAMPTZ NOT NULL  DEFAULT NOW()
);
