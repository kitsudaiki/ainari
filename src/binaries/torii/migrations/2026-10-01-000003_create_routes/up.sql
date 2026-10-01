CREATE TABLE routes (
    uuid VARCHAR(40) PRIMARY KEY,
    vni INTEGER NOT NULL DEFAULT 0,
    dest_ip VARCHAR(16),
    target_iface VARCHAR(256),
    gateway_ip VARCHAR(16),
    next_hop_ip VARCHAR(16),
    next_hop_mac VARCHAR(17),
    encrypted BOOLEAN NOT NULL DEFAULT FALSE,
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
