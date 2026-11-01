-- the removed entries can not be restored, so all remaining entries are active

ALTER TABLE secrets ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE secrets ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE secrets ADD COLUMN deleted_by VARCHAR(256);

ALTER TABLE public_keys ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE public_keys ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE public_keys ADD COLUMN deleted_by VARCHAR(256);
