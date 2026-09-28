CREATE TABLE tasks (
    uuid VARCHAR(40) PRIMARY KEY,
    description VARCHAR(256),
    resource_uuid VARCHAR(40),
    resource_type VARCHAR(32),
    task_type VARCHAR(32),
    task_state VARCHAR(32),
    queued_at VARCHAR(64),
    started_at VARCHAR(64),
    aborted_at VARCHAR(64),
    finished_at VARCHAR(64),
    messages TEXT,
    owner_id VARCHAR(256),
    project_id VARCHAR(256),
    created_by VARCHAR(256)
);
