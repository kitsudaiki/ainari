-- key-value-store of the MLS-client of the gateway: its identity, its key-packages and the
-- state of its groups, base64-encoded
CREATE TABLE mls_storage (
    storage_key TEXT PRIMARY KEY NOT NULL,
    storage_value TEXT NOT NULL
);
