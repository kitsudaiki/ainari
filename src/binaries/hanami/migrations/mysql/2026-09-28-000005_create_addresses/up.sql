-- binary collation, so comparisons and unique-checks are case-sensitive like in sqlite
CREATE TABLE addresses (
    uuid VARCHAR(40) PRIMARY KEY,
    mac_address VARCHAR(40),
    tap_name VARCHAR(16),
    internal_ip VARCHAR(40),
    network_uuid VARCHAR(40),
    vni INT NOT NULL DEFAULT 0,
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
    deleted_by VARCHAR(256),

    -- mysql has no partial indexes, so the unique indexes are built on generated columns, which
    -- are NULL for deleted entries. NULL never collides within a unique index, so deleted entries
    -- don't block their values, like the partial indexes of the sqlite-database.
    active_mac_address VARCHAR(40)
        GENERATED ALWAYS AS (IF(status = 'ACTIVE', mac_address, NULL)) VIRTUAL,
    active_tap_name VARCHAR(16)
        GENERATED ALWAYS AS (IF(status = 'ACTIVE', tap_name, NULL)) VIRTUAL,
    active_network_uuid VARCHAR(40)
        GENERATED ALWAYS AS (IF(status = 'ACTIVE', network_uuid, NULL)) VIRTUAL,

    UNIQUE INDEX addresses_active_mac_address (active_mac_address),
    UNIQUE INDEX addresses_active_tap_name (active_tap_name),
    UNIQUE INDEX addresses_active_network_internal_ip (active_network_uuid, internal_ip)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
