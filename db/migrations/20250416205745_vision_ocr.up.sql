-- Add up migration script here
ALTER TABLE media_extra ADD COLUMN vision_ocr_version INT NOT NULL DEFAULT -1;
ALTER TABLE media_extra ADD COLUMN vision_ocr_result TEXT DEFAULT NULL;