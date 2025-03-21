-- Add up migration script here
ALTER TABLE media ADD COLUMN media_type TEXT NOT NULL DEFAULT 'other';
UPDATE media SET media_type = 'photo' WHERE is_photo = TRUE;
UPDATE media SET media_type = 'video' WHERE is_photo = FALSE;
ALTER TABLE media DROP COLUMN is_photo;