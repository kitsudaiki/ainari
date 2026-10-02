CREATE TABLE user_project_mapping (
    project_id VARCHAR(256),
    user_id VARCHAR(256),
    role VARCHAR(64),
    status VARCHAR(8),
    created_at VARCHAR(64),
    created_by VARCHAR(256),
    updated_at VARCHAR(64),
    updated_by VARCHAR(256),
    deleted_at VARCHAR(64),
    deleted_by VARCHAR(256)
);

-- deleted entries stay in the table, so the pair has to be unique only within the active entries
CREATE UNIQUE INDEX user_project_mapping_active_project_user
    ON user_project_mapping (project_id, user_id) WHERE status = 'ACTIVE';
