-- migrations/001_initial.sql
-- PostgreSQL schema for PRMS.  Idempotent: IF NOT EXISTS / DO NOTHING throughout.
-- Applied once at startup by PgEntityStore::migrate().

CREATE TABLE IF NOT EXISTS crew_leads (
    id   TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS passengers (
    id         TEXT    PRIMARY KEY NOT NULL,
    name       TEXT    NOT NULL,
    tier       TEXT    NOT NULL,
    deleted_at BIGINT
);

CREATE TABLE IF NOT EXISTS resources (
    id         TEXT    PRIMARY KEY NOT NULL,
    name       TEXT    NOT NULL,
    category   TEXT    NOT NULL,
    min_tier   TEXT    NOT NULL,
    deleted_at BIGINT
);

CREATE TABLE IF NOT EXISTS usage_events (
    id                  TEXT   PRIMARY KEY NOT NULL,
    passenger_id        TEXT   NOT NULL,
    resource_id         TEXT   NOT NULL,
    tier_at_attempt     TEXT   NOT NULL,
    min_tier_at_attempt TEXT   NOT NULL,
    timestamp           BIGINT NOT NULL,
    outcome             TEXT   NOT NULL
);

CREATE TABLE IF NOT EXISTS admin_events (
    id          TEXT   PRIMARY KEY NOT NULL,
    actor_id    TEXT   NOT NULL,
    action      TEXT   NOT NULL,
    target_kind TEXT   NOT NULL,
    target_id   TEXT   NOT NULL,
    timestamp   BIGINT NOT NULL,
    details     TEXT
);

CREATE INDEX IF NOT EXISTS idx_usage_passenger  ON usage_events (passenger_id);
CREATE INDEX IF NOT EXISTS idx_usage_timestamp  ON usage_events (timestamp);
CREATE INDEX IF NOT EXISTS idx_admin_timestamp  ON admin_events (timestamp);
