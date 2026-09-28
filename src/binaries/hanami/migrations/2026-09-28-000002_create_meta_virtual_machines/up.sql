CREATE TABLE meta_virtual_machines (
    uuid VARCHAR(40) PRIMARY KEY,
    name VARCHAR(256),
    sakura_host_uuid VARCHAR(40),
    proxy_uuid VARCHAR(40),
    owner_id VARCHAR(256),
    project_id VARCHAR(256),
    status VARCHAR(8),
    created_at VARCHAR(64),
    created_by VARCHAR(256),
    updated_at VARCHAR(64),
    updated_by VARCHAR(256),
    deleted_at VARCHAR(64),
    deleted_by VARCHAR(256),
    number_of_cores BIGINT NOT NULL DEFAULT 0 CHECK (number_of_cores >= 0),
    memory_size BIGINT NOT NULL DEFAULT 0 CHECK (memory_size >= 0),
    disk_size BIGINT NOT NULL DEFAULT 0 CHECK (disk_size >= 0)
);
