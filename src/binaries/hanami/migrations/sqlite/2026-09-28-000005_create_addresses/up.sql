CREATE TABLE addresses (
    uuid VARCHAR(40) PRIMARY KEY,
    mac_address VARCHAR(40),
    tap_name VARCHAR(16),
    internal_ip VARCHAR(40),
    network_uuid VARCHAR(40),
    vni INTEGER NOT NULL DEFAULT 0,
    host_address VARCHAR(256),
    virtual_machine_uuid VARCHAR(40),
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

-- deleted entries stay in the table, so the values have to be unique only within the active entries
CREATE UNIQUE INDEX addresses_active_mac_address
    ON addresses (mac_address) WHERE status = 'ACTIVE';
CREATE UNIQUE INDEX addresses_active_tap_name
    ON addresses (tap_name) WHERE status = 'ACTIVE';
CREATE UNIQUE INDEX addresses_active_network_internal_ip
    ON addresses (network_uuid, internal_ip) WHERE status = 'ACTIVE';
