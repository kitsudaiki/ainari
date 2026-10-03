CREATE TABLE network_filters (
    uuid VARCHAR(40) PRIMARY KEY,
    virtual_machine_uuid VARCHAR(40),
    direction VARCHAR(8),
    ip_ranges TEXT NOT NULL DEFAULT '',
    ports TEXT NOT NULL DEFAULT '',
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

-- a virtual_machine has one packet-filter per direction. Deleted entries stay in the table, so the
-- combination has to be unique only within the active entries.
CREATE UNIQUE INDEX network_filters_active_virtual_machine_direction
    ON network_filters (virtual_machine_uuid, direction) WHERE status = 'ACTIVE';
