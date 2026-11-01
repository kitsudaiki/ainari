-- the removed entries can not be restored, so all remaining entries are active
ALTER TABLE network_filters
    DROP INDEX network_filters_virtual_machine_direction,
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE',
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256),
    ADD COLUMN active_virtual_machine_uuid VARCHAR(40)
        GENERATED ALWAYS AS (IF(status = 'ACTIVE', virtual_machine_uuid, NULL)) VIRTUAL,
    ADD UNIQUE INDEX network_filters_active_virtual_machine_direction
        (active_virtual_machine_uuid, direction);

ALTER TABLE addresses
    DROP INDEX addresses_mac_address,
    DROP INDEX addresses_tap_name,
    DROP INDEX addresses_network_internal_ip,
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE',
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256),
    ADD COLUMN active_mac_address VARCHAR(40)
        GENERATED ALWAYS AS (IF(status = 'ACTIVE', mac_address, NULL)) VIRTUAL,
    ADD COLUMN active_tap_name VARCHAR(16)
        GENERATED ALWAYS AS (IF(status = 'ACTIVE', tap_name, NULL)) VIRTUAL,
    ADD COLUMN active_network_uuid VARCHAR(40)
        GENERATED ALWAYS AS (IF(status = 'ACTIVE', network_uuid, NULL)) VIRTUAL,
    ADD UNIQUE INDEX addresses_active_mac_address (active_mac_address),
    ADD UNIQUE INDEX addresses_active_tap_name (active_tap_name),
    ADD UNIQUE INDEX addresses_active_network_internal_ip (active_network_uuid, internal_ip);

ALTER TABLE floating_ips
    DROP INDEX floating_ips_floating_ip_addr,
    DROP INDEX floating_ips_network_internal_ip_addr,
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE',
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256),
    ADD COLUMN active_floating_ip_addr VARCHAR(40)
        GENERATED ALWAYS AS (IF(status = 'ACTIVE', floating_ip_addr, NULL)) VIRTUAL,
    ADD COLUMN active_network_uuid VARCHAR(40)
        GENERATED ALWAYS AS (IF(status = 'ACTIVE', network_uuid, NULL)) VIRTUAL,
    ADD UNIQUE INDEX floating_ips_active_floating_ip_addr (active_floating_ip_addr),
    ADD UNIQUE INDEX floating_ips_active_network_internal_ip_addr
        (active_network_uuid, internal_ip_addr);

ALTER TABLE vm_types
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE',
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256);

-- the addresses still contain the tenant of their network
ALTER TABLE networks
    DROP COLUMN vni,
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE',
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256);

ALTER TABLE meta_virtual_machines
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE',
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256);

ALTER TABLE hosts
    ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE',
    ADD COLUMN deleted_at VARCHAR(64),
    ADD COLUMN deleted_by VARCHAR(256);
