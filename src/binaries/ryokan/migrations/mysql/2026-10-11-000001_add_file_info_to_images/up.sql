-- type and size of the unencrypted file, which are measured before the file is encrypted
ALTER TABLE images
    ADD COLUMN file_type VARCHAR(32) NOT NULL DEFAULT 'unknown' AFTER file_path,
    ADD COLUMN file_size BIGINT NOT NULL DEFAULT 0 AFTER file_type;
