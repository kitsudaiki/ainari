-- backup- and restore-jobs, which are processed in the background
-- binary collation, so comparisons and unique-checks are case-sensitive like in sqlite
CREATE TABLE jobs (
    uuid VARCHAR(40) PRIMARY KEY,
    job_type VARCHAR(16) NOT NULL,
    backup_uuid VARCHAR(40) NOT NULL,
    project_id VARCHAR(256) NOT NULL,
    job_state VARCHAR(16) NOT NULL,
    step VARCHAR(256) NOT NULL,
    messages LONGTEXT NOT NULL,
    options LONGTEXT NOT NULL,
    owner_id VARCHAR(256) NOT NULL,
    created_at VARCHAR(64) NOT NULL,
    created_by VARCHAR(256) NOT NULL,
    started_at VARCHAR(64),
    finished_at VARCHAR(64)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
