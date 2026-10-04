CREATE TABLE key_packages (
    uuid VARCHAR(40) PRIMARY KEY,
    client_id VARCHAR(256),
    key_package TEXT,
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
CREATE INDEX key_packages_client_id ON key_packages (client_id, status);
