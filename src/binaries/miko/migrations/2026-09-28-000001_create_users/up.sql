CREATE TABLE users (
    id VARCHAR(256),
    name VARCHAR(256),
    is_admin VARCHAR(8),
    pw_hash VARCHAR(64),
    salt VARCHAR(64),
    projects TEXT,
    status VARCHAR(8),
    created_at VARCHAR(64),
    created_by VARCHAR(256),
    updated_at VARCHAR(64),
    updated_by VARCHAR(256),
    deleted_at VARCHAR(64),
    deleted_by VARCHAR(256)
);
