-- binary collation, so comparisons and unique-checks are case-sensitive like in sqlite
CREATE TABLE quotas (
    id VARCHAR(256),
    max_virtual_machine INT,
    max_image INT,
    max_secret INT,
    max_network INT,
    max_floating_ip INT,
    status VARCHAR(8),
    created_at VARCHAR(64),
    created_by VARCHAR(256),
    updated_at VARCHAR(64),
    updated_by VARCHAR(256),
    deleted_at VARCHAR(64),
    deleted_by VARCHAR(64)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
