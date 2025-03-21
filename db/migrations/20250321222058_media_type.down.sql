-- Add down migration script here
ALTER TABLE media ADD COLUMN is_photo BOOLEAN NOT NULL DEFAULT FALSE;
UPDATE media SET is_photo = TRUE WHERE media_type = 'photo';
UPDATE media SET is_photo = FALSE WHERE media_type = 'video';
ALTER TABLE media DROP COLUMN media_type;