-- Add down migration script here
ALTER TABLE media DROP COLUMN has_thumbnail;
ALTER TABLE media RENAME COLUMN thumbnail_version TO thumbnail_version_old;
ALTER TABLE media ADD COLUMN thumbnail_version INT NOT NULL DEFAULT 0;
UPDATE media SET thumbnail_version = thumbnail_version_old;
ALTER TABLE media DROP COLUMN thumbnail_version_old;