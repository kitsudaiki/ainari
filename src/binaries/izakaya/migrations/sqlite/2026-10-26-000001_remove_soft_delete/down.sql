-- the removed entries can not be restored, so all remaining entries are active
DROP INDEX mls_messages_recipient;
ALTER TABLE mls_messages ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE mls_messages ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE mls_messages ADD COLUMN deleted_by VARCHAR(256);
CREATE INDEX mls_messages_recipient ON mls_messages (recipient, status);

DROP INDEX key_packages_client_id;
ALTER TABLE key_packages ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE key_packages ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE key_packages ADD COLUMN deleted_by VARCHAR(256);
CREATE INDEX key_packages_client_id ON key_packages (client_id, status);
