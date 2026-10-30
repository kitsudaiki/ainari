-- the removed entries can not be restored, so all remaining entries are active
DROP INDEX network_filters_virtual_machine_direction;
ALTER TABLE network_filters ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE network_filters ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE network_filters ADD COLUMN deleted_by VARCHAR(256);
CREATE UNIQUE INDEX network_filters_active_virtual_machine_direction
    ON network_filters (virtual_machine_uuid, direction) WHERE status = 'ACTIVE';

DROP INDEX addresses_mac_address;
DROP INDEX addresses_tap_name;
DROP INDEX addresses_network_internal_ip;
ALTER TABLE addresses ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE addresses ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE addresses ADD COLUMN deleted_by VARCHAR(256);
CREATE UNIQUE INDEX addresses_active_mac_address
    ON addresses (mac_address) WHERE status = 'ACTIVE';
CREATE UNIQUE INDEX addresses_active_tap_name
    ON addresses (tap_name) WHERE status = 'ACTIVE';
CREATE UNIQUE INDEX addresses_active_network_internal_ip
    ON addresses (network_uuid, internal_ip) WHERE status = 'ACTIVE';

DROP INDEX floating_ips_floating_ip_addr;
DROP INDEX floating_ips_network_internal_ip_addr;
ALTER TABLE floating_ips ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE floating_ips ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE floating_ips ADD COLUMN deleted_by VARCHAR(256);
CREATE UNIQUE INDEX floating_ips_active_floating_ip_addr
    ON floating_ips (floating_ip_addr) WHERE status = 'ACTIVE';
CREATE UNIQUE INDEX floating_ips_active_network_internal_ip_addr
    ON floating_ips (network_uuid, internal_ip_addr) WHERE status = 'ACTIVE';

ALTER TABLE vm_types ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE vm_types ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE vm_types ADD COLUMN deleted_by VARCHAR(256);

ALTER TABLE networks ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE networks ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE networks ADD COLUMN deleted_by VARCHAR(256);
-- the addresses still contain the tenant of their network
ALTER TABLE networks DROP COLUMN vni;

ALTER TABLE meta_virtual_machines ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE meta_virtual_machines ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE meta_virtual_machines ADD COLUMN deleted_by VARCHAR(256);

ALTER TABLE hosts ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE hosts ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE hosts ADD COLUMN deleted_by VARCHAR(256);
