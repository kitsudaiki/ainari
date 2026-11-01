-- the removed entries can not be restored, so all remaining entries are active
ALTER TABLE mls_messages
    DROP INDEX mls_messages_recipient,
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE',
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256),
    ADD INDEX mls_messages_recipient (recipient, status);

ALTER TABLE key_packages
    DROP INDEX key_packages_client_id,
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE',
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256),
    ADD INDEX key_packages_client_id (client_id, status);
