-- binary collation, so comparisons and unique-checks are case-sensitive like in sqlite
CREATE TABLE hosts (
    uuid VARCHAR(40) PRIMARY KEY,
    name VARCHAR(256),
    address VARCHAR(256),
    number_of_cores BIGINT NOT NULL DEFAULT 0 CHECK (number_of_cores >= 0),
    used_number_of_cores BIGINT NOT NULL DEFAULT 0 CHECK (used_number_of_cores >= 0),
    memory_size BIGINT NOT NULL DEFAULT 0 CHECK (memory_size >= 0),
    amount_of_used_memory BIGINT NOT NULL DEFAULT 0 CHECK (amount_of_used_memory >= 0),
    disk_space BIGINT NOT NULL DEFAULT 0 CHECK (disk_space >= 0),
    amount_of_used_disk_space BIGINT NOT NULL DEFAULT 0 CHECK (amount_of_used_disk_space >= 0),
    status VARCHAR(8),
    created_at VARCHAR(64),
    created_by VARCHAR(256),
    updated_at VARCHAR(64),
    updated_by VARCHAR(256),
    deleted_at VARCHAR(64),
    deleted_by VARCHAR(256)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
