-- binary collation, so comparisons and unique-checks are case-sensitive like in sqlite
CREATE TABLE key_packages (
    uuid VARCHAR(40) PRIMARY KEY,
    client_id VARCHAR(256),
    key_package LONGTEXT,
    owner_id VARCHAR(256),
    project_id VARCHAR(256),
    status VARCHAR(8),
    created_at VARCHAR(64),
    created_by VARCHAR(256),
    updated_at VARCHAR(64),
    updated_by VARCHAR(256),
    deleted_at VARCHAR(64),
    deleted_by VARCHAR(256),
    INDEX key_packages_client_id (client_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
