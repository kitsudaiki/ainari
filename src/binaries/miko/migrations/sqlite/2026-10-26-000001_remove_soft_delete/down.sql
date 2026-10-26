-- the removed entries can not be restored, so all remaining entries are active
DROP INDEX user_project_mapping_project_user;
ALTER TABLE user_project_mapping ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE user_project_mapping ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE user_project_mapping ADD COLUMN deleted_by VARCHAR(256);
CREATE UNIQUE INDEX user_project_mapping_active_project_user
    ON user_project_mapping (project_id, user_id) WHERE status = 'ACTIVE';

ALTER TABLE quotas ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE quotas ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE quotas ADD COLUMN deleted_by VARCHAR(64);

ALTER TABLE projects ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE projects ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE projects ADD COLUMN deleted_by VARCHAR(256);

ALTER TABLE users ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE users ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE users ADD COLUMN deleted_by VARCHAR(256);
