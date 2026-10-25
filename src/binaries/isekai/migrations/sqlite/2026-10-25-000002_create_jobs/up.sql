-- backup- and restore-jobs, which are processed in the background
CREATE TABLE jobs (
    uuid VARCHAR(40) PRIMARY KEY,
    job_type VARCHAR(16) NOT NULL,
    backup_uuid VARCHAR(40) NOT NULL,
    project_id VARCHAR(256) NOT NULL,
    job_state VARCHAR(16) NOT NULL,
    step VARCHAR(256) NOT NULL,
    messages TEXT NOT NULL,
    options TEXT NOT NULL,
    owner_id VARCHAR(256) NOT NULL,
    created_at VARCHAR(64) NOT NULL,
    created_by VARCHAR(256) NOT NULL,
    started_at VARCHAR(64),
    finished_at VARCHAR(64)
);
