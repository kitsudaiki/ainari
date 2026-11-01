-- deleted entries are removed from the tables now, instead of being only marked as deleted, so
-- the entries, which were marked as deleted before, are removed together with the columns of the
-- soft-delete

DELETE FROM hosts WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE hosts DROP COLUMN status;
ALTER TABLE hosts DROP COLUMN deleted_at;
ALTER TABLE hosts DROP COLUMN deleted_by;

DELETE FROM images WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE images DROP COLUMN status;
ALTER TABLE images DROP COLUMN deleted_at;
ALTER TABLE images DROP COLUMN deleted_by;

DELETE FROM backups WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE backups DROP COLUMN status;
ALTER TABLE backups DROP COLUMN deleted_at;
ALTER TABLE backups DROP COLUMN deleted_by;
