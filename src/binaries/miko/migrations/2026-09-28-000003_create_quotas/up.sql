CREATE TABLE quotas (
    id VARCHAR(256),
    max_virtual_machine INTEGER,
    max_image INTEGER,
    max_secret INTEGER,
    max_network INTEGER,
    max_floating_ip INTEGER,
    status VARCHAR(8),
    created_at VARCHAR(64),
    created_by VARCHAR(256),
    updated_at VARCHAR(64),
    updated_by VARCHAR(256),
    deleted_at VARCHAR(64),
    deleted_by VARCHAR(64)
);
