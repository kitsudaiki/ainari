-- binary collation, so comparisons and unique-checks are case-sensitive like in sqlite
CREATE TABLE floating_ips (
    uuid VARCHAR(40) PRIMARY KEY,
    name VARCHAR(256) NOT NULL DEFAULT '',
    network_uuid VARCHAR(40),
    internal_ip_addr VARCHAR(40),
    floating_ip_addr VARCHAR(40),
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
    active_floating_ip_addr VARCHAR(40)
        GENERATED ALWAYS AS (IF(status = 'ACTIVE', floating_ip_addr, NULL)) VIRTUAL,
    active_network_uuid VARCHAR(40)
        GENERATED ALWAYS AS (IF(status = 'ACTIVE', network_uuid, NULL)) VIRTUAL,

    UNIQUE INDEX floating_ips_active_floating_ip_addr (active_floating_ip_addr),
    -- The torii keys the outbound translation by the internal IP-address, so a virtual_machine can
    -- only have one floating IP-address. Detached entries have NULL as internal IP-address, which
    -- never collides within a unique index.
    UNIQUE INDEX floating_ips_active_network_internal_ip_addr (active_network_uuid, internal_ip_addr)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
