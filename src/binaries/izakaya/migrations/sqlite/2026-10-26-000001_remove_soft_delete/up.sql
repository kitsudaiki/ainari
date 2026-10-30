-- deleted entries are removed from the tables now, instead of being only marked as deleted, so
-- the entries, which were marked as deleted before, are removed together with the columns of the
-- soft-delete. The indexes contain the status, so they are replaced by indexes without it.
DROP INDEX key_packages_client_id;
DELETE FROM key_packages WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE key_packages DROP COLUMN status;
ALTER TABLE key_packages DROP COLUMN deleted_at;
ALTER TABLE key_packages DROP COLUMN deleted_by;
CREATE INDEX key_packages_client_id ON key_packages (client_id);

DROP INDEX mls_messages_recipient;
DELETE FROM mls_messages WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE mls_messages DROP COLUMN status;
ALTER TABLE mls_messages DROP COLUMN deleted_at;
ALTER TABLE mls_messages DROP COLUMN deleted_by;
CREATE INDEX mls_messages_recipient ON mls_messages (recipient);
