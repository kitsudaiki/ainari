-- backups, which were created or imported by isekai. The files of a backup are stored as one
-- package on a backup-target, which is tracked by the backup-resource with the same UUID in ryokan.
CREATE TABLE backups (
    uuid VARCHAR(40) PRIMARY KEY,
    name VARCHAR(256) NOT NULL,
    backup_kind VARCHAR(32) NOT NULL,
    project_id VARCHAR(256) NOT NULL,
    virtual_machine_uuid VARCHAR(40),
    backup_state VARCHAR(16) NOT NULL,
    ainari_version VARCHAR(64) NOT NULL,
    secret_uuids TEXT NOT NULL,
    owner_id VARCHAR(256),
    status VARCHAR(8),
    created_at VARCHAR(64),
    created_by VARCHAR(256),
    updated_at VARCHAR(64),
    updated_by VARCHAR(256),
    deleted_at VARCHAR(64),
    deleted_by VARCHAR(256)
);
