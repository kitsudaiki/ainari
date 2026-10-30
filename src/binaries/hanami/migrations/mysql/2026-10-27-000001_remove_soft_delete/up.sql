-- deleted entries are removed from the tables now, instead of being only marked as deleted, so
-- the entries, which were marked as deleted before, are removed together with the columns of the
-- soft-delete

-- the tenant of a network was only stored within its addresses, also within the deleted ones, so
-- it is moved to the network itself, before the deleted addresses are removed
ALTER TABLE networks ADD COLUMN vni INT;
UPDATE networks SET vni = (
    SELECT addresses.vni FROM addresses WHERE addresses.network_uuid = networks.uuid LIMIT 1
);

DELETE FROM hosts WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE hosts DROP COLUMN status, DROP COLUMN deleted_at, DROP COLUMN deleted_by;

DELETE FROM meta_virtual_machines WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE meta_virtual_machines DROP COLUMN status, DROP COLUMN deleted_at, DROP COLUMN deleted_by;

DELETE FROM networks WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE networks DROP COLUMN status, DROP COLUMN deleted_at, DROP COLUMN deleted_by;

DELETE FROM vm_types WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE vm_types DROP COLUMN status, DROP COLUMN deleted_at, DROP COLUMN deleted_by;

-- the generated columns for the unique indexes depend on the status, so the indexes are replaced
-- by indexes over all entries
DELETE FROM floating_ips WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE floating_ips
    DROP INDEX floating_ips_active_floating_ip_addr,
    DROP INDEX floating_ips_active_network_internal_ip_addr,
    DROP COLUMN active_floating_ip_addr,
    DROP COLUMN active_network_uuid,
    DROP COLUMN status,
    DROP COLUMN deleted_at,
    DROP COLUMN deleted_by,
    ADD UNIQUE INDEX floating_ips_floating_ip_addr (floating_ip_addr),
    -- detached entries have NULL as internal IP-address, which never collides within a unique
    -- index
    ADD UNIQUE INDEX floating_ips_network_internal_ip_addr (network_uuid, internal_ip_addr);

DELETE FROM addresses WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE addresses
    DROP INDEX addresses_active_mac_address,
    DROP INDEX addresses_active_tap_name,
    DROP INDEX addresses_active_network_internal_ip,
    DROP COLUMN active_mac_address,
    DROP COLUMN active_tap_name,
    DROP COLUMN active_network_uuid,
    DROP COLUMN status,
    DROP COLUMN deleted_at,
    DROP COLUMN deleted_by,
    ADD UNIQUE INDEX addresses_mac_address (mac_address),
    ADD UNIQUE INDEX addresses_tap_name (tap_name),
    ADD UNIQUE INDEX addresses_network_internal_ip (network_uuid, internal_ip);

DELETE FROM network_filters WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE network_filters
    DROP INDEX network_filters_active_virtual_machine_direction,
    DROP COLUMN active_virtual_machine_uuid,
    DROP COLUMN status,
    DROP COLUMN deleted_at,
    DROP COLUMN deleted_by,
    -- a virtual_machine has one packet-filter per direction
    ADD UNIQUE INDEX network_filters_virtual_machine_direction (virtual_machine_uuid, direction);
