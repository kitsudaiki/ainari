CREATE TABLE taps (
    uuid VARCHAR(40) PRIMARY KEY,
    tap_name VARCHAR(256),
    vni INTEGER NOT NULL DEFAULT 0,
    vm_mac VARCHAR(17),
    vm_ip VARCHAR(16),
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
