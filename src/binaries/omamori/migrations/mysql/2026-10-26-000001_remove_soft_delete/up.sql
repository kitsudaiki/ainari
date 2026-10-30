-- deleted entries are removed from the tables now, instead of being only marked as deleted, so
-- the entries, which were marked as deleted before, are removed together with the columns of the
-- soft-delete

DELETE FROM secrets WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE secrets DROP COLUMN status, DROP COLUMN deleted_at, DROP COLUMN deleted_by;

DELETE FROM public_keys WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE public_keys DROP COLUMN status, DROP COLUMN deleted_at, DROP COLUMN deleted_by;
