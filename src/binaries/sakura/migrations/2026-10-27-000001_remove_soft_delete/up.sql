-- deleted entries are removed from the table now, instead of being only marked as deleted, so
-- the entries, which were marked as deleted before, are removed together with the columns of the
-- soft-delete
DELETE FROM virtual_machines WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE virtual_machines DROP COLUMN status;
ALTER TABLE virtual_machines DROP COLUMN deleted_at;
ALTER TABLE virtual_machines DROP COLUMN deleted_by;
