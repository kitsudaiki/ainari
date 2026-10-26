-- deleted entries are removed from the tables now, instead of being only marked as deleted, so
-- the entries, which were marked as deleted before, are removed together with the columns of the
-- soft-delete
DELETE FROM users WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE users DROP COLUMN status, DROP COLUMN deleted_at, DROP COLUMN deleted_by;

DELETE FROM projects WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE projects DROP COLUMN status, DROP COLUMN deleted_at, DROP COLUMN deleted_by;

DELETE FROM quotas WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE quotas DROP COLUMN status, DROP COLUMN deleted_at, DROP COLUMN deleted_by;

-- the generated column for the unique index depends on the status, so the index is replaced by
-- an index over all entries
DELETE FROM user_project_mapping WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE user_project_mapping
    DROP INDEX user_project_mapping_active_project_user,
    DROP COLUMN active_project_id,
    DROP COLUMN status,
    DROP COLUMN deleted_at,
    DROP COLUMN deleted_by,
    ADD UNIQUE INDEX user_project_mapping_project_user (project_id, user_id);
