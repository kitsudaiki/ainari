-- storage-backend of an onsen-host, which the host reports on every registration. All existing
-- hosts use the local backend without volumes, until they register again.
ALTER TABLE hosts ADD COLUMN backend_type VARCHAR(16) NOT NULL DEFAULT 'LOCAL';
ALTER TABLE hosts ADD COLUMN supports_volumes BOOLEAN NOT NULL DEFAULT FALSE;
