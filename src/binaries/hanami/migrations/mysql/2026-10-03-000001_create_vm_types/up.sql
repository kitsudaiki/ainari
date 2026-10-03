-- binary collation, so comparisons and unique-checks are case-sensitive like in sqlite
CREATE TABLE vm_types (
    uuid VARCHAR(40) PRIMARY KEY,
    name VARCHAR(256),
    number_of_cores INTEGER NOT NULL DEFAULT 0 CHECK (number_of_cores >= 0),
    amount_of_memory BIGINT NOT NULL DEFAULT 0 CHECK (amount_of_memory >= 0),
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
