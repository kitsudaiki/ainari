CREATE TABLE network_interfaces (
    uuid VARCHAR(40) PRIMARY KEY,
    iface_name VARCHAR(256),
    ip_cidr VARCHAR(64),
    up BOOLEAN NOT NULL DEFAULT FALSE,
    vni INTEGER NOT NULL DEFAULT 0,
    fip_port BOOLEAN NOT NULL DEFAULT FALSE,
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
