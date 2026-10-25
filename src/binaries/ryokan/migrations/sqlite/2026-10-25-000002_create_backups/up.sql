-- packages of backups, which are stored as one directory on a backup-target. The single files of
-- a backup are not tracked, only the package as a whole.
CREATE TABLE backups (
    uuid VARCHAR(40) PRIMARY KEY,
    host_uuid VARCHAR(40) NOT NULL,
    directory_path TEXT NOT NULL,
    backup_status VARCHAR(16) NOT NULL,
    file_count INTEGER NOT NULL DEFAULT 0,
    total_size BIGINT NOT NULL DEFAULT 0,
    manifest_sha256 VARCHAR(64),
    owner_id VARCHAR(256),
    project_id VARCHAR(256),
    status VARCHAR(8),
    created_at VARCHAR(64),
    created_by VARCHAR(256),
    updated_at VARCHAR(64),
    updated_by VARCHAR(256),
    deleted_at VARCHAR(64),
    deleted_by VARCHAR(256)
);
