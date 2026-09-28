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
    deleted_by VARCHAR(256)
);

-- deleted entries stay in the table, so the floating IP-address has to be unique only within the
-- active entries
CREATE UNIQUE INDEX floating_ips_active_floating_ip_addr
    ON floating_ips (floating_ip_addr) WHERE status = 'ACTIVE';

-- The torii keys the outbound translation by the internal IP-address, so a virtual_machine can
-- only have one floating IP-address. Detached entries have NULL as internal IP-address, which
-- never collides within a unique index.
CREATE UNIQUE INDEX floating_ips_active_network_internal_ip_addr
    ON floating_ips (network_uuid, internal_ip_addr) WHERE status = 'ACTIVE';
