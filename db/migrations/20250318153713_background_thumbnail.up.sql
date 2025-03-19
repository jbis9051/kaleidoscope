-- Add up migration script here
ALTER TABLE media ADD COLUMN has_thumbnail BOOLEAN DEFAULT FALSE;

UPDATE media SET has_thumbnail = TRUE; -- we know all media has a thumbnail

-- modify thumbnail_version to have default value -1
ALTER TABLE media RENAME COLUMN thumbnail_version TO thumbnail_version_old;
ALTER TABLE media ADD COLUMN thumbnail_version INT NOT NULL DEFAULT -1;
UPDATE media SET thumbnail_version = thumbnail_version_old;
ALTER TABLE media DROP COLUMN thumbnail_version_old;