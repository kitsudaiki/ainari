-- deleted entries are removed from the tables now, instead of being only marked as deleted, so
-- the entries, which were marked as deleted before, are removed together with the columns of the
-- soft-delete. The indexes contain the status, so they are replaced by indexes without it.
DELETE FROM key_packages WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE key_packages
    DROP INDEX key_packages_client_id,
    DROP COLUMN status,
    DROP COLUMN deleted_at,
    DROP COLUMN deleted_by,
    ADD INDEX key_packages_client_id (client_id);

DELETE FROM mls_messages WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE mls_messages
    DROP INDEX mls_messages_recipient,
    DROP COLUMN status,
    DROP COLUMN deleted_at,
    DROP COLUMN deleted_by,
    ADD INDEX mls_messages_recipient (recipient);
