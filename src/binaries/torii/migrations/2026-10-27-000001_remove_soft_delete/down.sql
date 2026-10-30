-- the removed entries can not be restored, so all remaining entries are active

ALTER TABLE floating_ips ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE floating_ips ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE floating_ips ADD COLUMN deleted_by VARCHAR(256);

ALTER TABLE network_filters ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE network_filters ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE network_filters ADD COLUMN deleted_by VARCHAR(256);

ALTER TABLE routes ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE routes ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE routes ADD COLUMN deleted_by VARCHAR(256);

ALTER TABLE taps ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE taps ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE taps ADD COLUMN deleted_by VARCHAR(256);

ALTER TABLE network_interfaces ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE network_interfaces ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE network_interfaces ADD COLUMN deleted_by VARCHAR(256);

ALTER TABLE proxys ADD COLUMN status VARCHAR(8) DEFAULT 'ACTIVE';
ALTER TABLE proxys ADD COLUMN deleted_at VARCHAR(64);
ALTER TABLE proxys ADD COLUMN deleted_by VARCHAR(256);
