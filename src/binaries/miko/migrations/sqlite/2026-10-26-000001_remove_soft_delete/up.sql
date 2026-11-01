-- deleted entries are removed from the tables now, instead of being only marked as deleted, so
-- the entries, which were marked as deleted before, are removed together with the columns of the
-- soft-delete
DELETE FROM users WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE users DROP COLUMN status;
ALTER TABLE users DROP COLUMN deleted_at;
ALTER TABLE users DROP COLUMN deleted_by;

DELETE FROM projects WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE projects DROP COLUMN status;
ALTER TABLE projects DROP COLUMN deleted_at;
ALTER TABLE projects DROP COLUMN deleted_by;

DELETE FROM quotas WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE quotas DROP COLUMN status;
ALTER TABLE quotas DROP COLUMN deleted_at;
ALTER TABLE quotas DROP COLUMN deleted_by;

-- the partial index depends on the status, so it is replaced by an index over all entries
DROP INDEX user_project_mapping_active_project_user;
DELETE FROM user_project_mapping WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE user_project_mapping DROP COLUMN status;
ALTER TABLE user_project_mapping DROP COLUMN deleted_at;
ALTER TABLE user_project_mapping DROP COLUMN deleted_by;
CREATE UNIQUE INDEX user_project_mapping_project_user
    ON user_project_mapping (project_id, user_id);
