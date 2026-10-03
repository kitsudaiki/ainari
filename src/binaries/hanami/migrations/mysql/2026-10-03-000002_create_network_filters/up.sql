-- binary collation, so comparisons and unique-checks are case-sensitive like in sqlite
CREATE TABLE network_filters (
    uuid VARCHAR(40) PRIMARY KEY,
    virtual_machine_uuid VARCHAR(40),
    direction VARCHAR(8),
    ip_ranges TEXT NOT NULL,
    ports TEXT NOT NULL,
    owner_id VARCHAR(256),
    project_id VARCHAR(256),
    status VARCHAR(8),
    created_at VARCHAR(64),
    created_by VARCHAR(256),
    updated_at VARCHAR(64),
    updated_by VARCHAR(256),
    deleted_at VARCHAR(64),
    deleted_by VARCHAR(256),

    -- mysql has no partial indexes, so the unique index is built on a generated column, which is
    -- NULL for deleted entries. NULL never collides within a unique index, so deleted entries
    -- don't block their values, like the partial index of the sqlite-database.
    active_virtual_machine_uuid VARCHAR(40)
        GENERATED ALWAYS AS (IF(status = 'ACTIVE', virtual_machine_uuid, NULL)) VIRTUAL,

    -- a virtual_machine has one packet-filter per direction
    UNIQUE INDEX network_filters_active_virtual_machine_direction (active_virtual_machine_uuid, direction)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
