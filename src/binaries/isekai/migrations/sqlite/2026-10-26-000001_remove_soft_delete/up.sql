-- deleted entries are removed from the tables now, instead of being only marked as deleted, so
-- the entries, which were marked as deleted before, are removed together with the columns of the
-- soft-delete

DELETE FROM backups WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE backups DROP COLUMN status;
ALTER TABLE backups DROP COLUMN deleted_at;
ALTER TABLE backups DROP COLUMN deleted_by;
