-- Add down migration script here
ALTER TABLE media_extra DROP COLUMN vision_ocr_version;
ALTER TABLE media_extra DROP COLUMN vision_ocr_result;