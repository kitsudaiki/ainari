-- the removed entries can not be restored, so all remaining entries are active
ALTER TABLE virtual_machines ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE virtual_machines ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE virtual_machines ADD COLUMN deleted_by VARCHAR(256);
