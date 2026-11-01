-- deleted entries are removed from the tables now, instead of being only marked as deleted, so
-- the entries, which were marked as deleted before, are removed together with the columns of the
-- soft-delete

-- the tenant of a network was only stored within its addresses, also within the deleted ones, so
-- it is moved to the network itself, before the deleted addresses are removed
ALTER TABLE networks ADD COLUMN vni INTEGER;
UPDATE networks SET vni = (
    SELECT addresses.vni FROM addresses WHERE addresses.network_uuid = networks.uuid LIMIT 1
);

DELETE FROM hosts WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE hosts DROP COLUMN status;
ALTER TABLE hosts DROP COLUMN deleted_at;
ALTER TABLE hosts DROP COLUMN deleted_by;

DELETE FROM meta_virtual_machines WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE meta_virtual_machines DROP COLUMN status;
ALTER TABLE meta_virtual_machines DROP COLUMN deleted_at;
ALTER TABLE meta_virtual_machines DROP COLUMN deleted_by;

DELETE FROM networks WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE networks DROP COLUMN status;
ALTER TABLE networks DROP COLUMN deleted_at;
ALTER TABLE networks DROP COLUMN deleted_by;

DELETE FROM vm_types WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE vm_types DROP COLUMN status;
ALTER TABLE vm_types DROP COLUMN deleted_at;
ALTER TABLE vm_types DROP COLUMN deleted_by;

-- the partial indexes depend on the status, so they are replaced by indexes over all entries
DROP INDEX floating_ips_active_floating_ip_addr;
DROP INDEX floating_ips_active_network_internal_ip_addr;
DELETE FROM floating_ips WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE floating_ips DROP COLUMN status;
ALTER TABLE floating_ips DROP COLUMN deleted_at;
ALTER TABLE floating_ips DROP COLUMN deleted_by;
CREATE UNIQUE INDEX floating_ips_floating_ip_addr ON floating_ips (floating_ip_addr);
-- detached entries have NULL as internal IP-address, which never collides within a unique index
CREATE UNIQUE INDEX floating_ips_network_internal_ip_addr
    ON floating_ips (network_uuid, internal_ip_addr);

DROP INDEX addresses_active_mac_address;
DROP INDEX addresses_active_tap_name;
DROP INDEX addresses_active_network_internal_ip;
DELETE FROM addresses WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE addresses DROP COLUMN status;
ALTER TABLE addresses DROP COLUMN deleted_at;
ALTER TABLE addresses DROP COLUMN deleted_by;
CREATE UNIQUE INDEX addresses_mac_address ON addresses (mac_address);
CREATE UNIQUE INDEX addresses_tap_name ON addresses (tap_name);
CREATE UNIQUE INDEX addresses_network_internal_ip ON addresses (network_uuid, internal_ip);

DROP INDEX network_filters_active_virtual_machine_direction;
DELETE FROM network_filters WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE network_filters DROP COLUMN status;
ALTER TABLE network_filters DROP COLUMN deleted_at;
ALTER TABLE network_filters DROP COLUMN deleted_by;
-- a virtual_machine has one packet-filter per direction
CREATE UNIQUE INDEX network_filters_virtual_machine_direction
    ON network_filters (virtual_machine_uuid, direction);
