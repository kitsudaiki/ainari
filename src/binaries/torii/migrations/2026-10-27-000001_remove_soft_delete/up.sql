-- deleted entries are removed from the tables now, instead of being only marked as deleted, so
-- the entries, which were marked as deleted before, are removed together with the columns of the
-- soft-delete

DELETE FROM proxys WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE proxys DROP COLUMN status;
ALTER TABLE proxys DROP COLUMN deleted_at;
ALTER TABLE proxys DROP COLUMN deleted_by;

DELETE FROM network_interfaces WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE network_interfaces DROP COLUMN status;
ALTER TABLE network_interfaces DROP COLUMN deleted_at;
ALTER TABLE network_interfaces DROP COLUMN deleted_by;

DELETE FROM taps WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE taps DROP COLUMN status;
ALTER TABLE taps DROP COLUMN deleted_at;
ALTER TABLE taps DROP COLUMN deleted_by;

DELETE FROM routes WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE routes DROP COLUMN status;
ALTER TABLE routes DROP COLUMN deleted_at;
ALTER TABLE routes DROP COLUMN deleted_by;

DELETE FROM network_filters WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE network_filters DROP COLUMN status;
ALTER TABLE network_filters DROP COLUMN deleted_at;
ALTER TABLE network_filters DROP COLUMN deleted_by;

DELETE FROM floating_ips WHERE status IS NULL OR status <> 'ACTIVE';
ALTER TABLE floating_ips DROP COLUMN status;
ALTER TABLE floating_ips DROP COLUMN deleted_at;
ALTER TABLE floating_ips DROP COLUMN deleted_by;
