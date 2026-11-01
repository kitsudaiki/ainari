-- the removed entries can not be restored, so all remaining entries are active
ALTER TABLE user_project_mapping
    DROP INDEX user_project_mapping_project_user,
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE' AFTER role,
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256),
    ADD COLUMN active_project_id VARCHAR(256)
        GENERATED ALWAYS AS (IF(status = 'ACTIVE', project_id, NULL)) VIRTUAL,
    ADD UNIQUE INDEX user_project_mapping_active_project_user (active_project_id, user_id);

ALTER TABLE quotas
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE' AFTER max_floating_ip,
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(64);

ALTER TABLE projects
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE' AFTER name,
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256);

ALTER TABLE users
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE' AFTER salt,
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256);
