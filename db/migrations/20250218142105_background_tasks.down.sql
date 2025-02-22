-- Add down migration script here
DELETE FROM kv WHERE key = 'migration_version';

DROP TABLE queue;