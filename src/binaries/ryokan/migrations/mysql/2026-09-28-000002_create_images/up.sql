-- binary collation, so comparisons and unique-checks are case-sensitive like in sqlite
CREATE TABLE images (
    uuid VARCHAR(40) PRIMARY KEY,
    name VARCHAR(256),
    onsen_address VARCHAR(256),
    file_path LONGTEXT,
    secret_uuid VARCHAR(40),
    is_snapshot BOOLEAN NOT NULL DEFAULT FALSE,
    owner_id VARCHAR(256),
    project_id VARCHAR(256),
    status VARCHAR(8),
    created_at VARCHAR(64),
    created_by VARCHAR(256),
    updated_at VARCHAR(64),
    updated_by VARCHAR(256),
    deleted_at VARCHAR(64),
    deleted_by VARCHAR(256)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
