-- Add down migration script here
ALTER TABLE custom_metadata DROP COLUMN include_search;
DROP TABLE custom_task_media;