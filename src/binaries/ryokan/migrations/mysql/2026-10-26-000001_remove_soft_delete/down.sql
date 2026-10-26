-- the removed entries can not be restored, so all remaining entries are active

ALTER TABLE hosts
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE',
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256);

ALTER TABLE images
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE',
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256);

ALTER TABLE backups
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE',
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256);
