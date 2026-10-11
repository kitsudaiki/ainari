-- type and size of the unencrypted file, which are measured before the file is encrypted. Sqlite
-- can only append new columns at the end of the table.
ALTER TABLE images ADD COLUMN file_type VARCHAR(32) NOT NULL DEFAULT 'unknown';
ALTER TABLE images ADD COLUMN file_size BIGINT NOT NULL DEFAULT 0;
