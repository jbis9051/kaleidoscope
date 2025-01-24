-- Add down migration script here
ALTER TABLE media DROP COLUMN longitude;
ALTER TABLE media DROP COLUMN latitude;

ALTER TABLE media DROP COLUMN metadata_version;
ALTER TABLE media DROP COLUMN thumbnail_version;