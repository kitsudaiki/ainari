-- state of the file of an image (UPLOADING, ENCRYPTING, READY, IN_USE). All existing images are
-- complete, so they are READY.
ALTER TABLE images ADD COLUMN image_status VARCHAR(16) NOT NULL DEFAULT 'READY';
