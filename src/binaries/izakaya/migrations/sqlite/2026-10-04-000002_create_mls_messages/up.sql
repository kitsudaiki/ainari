CREATE TABLE mls_messages (
    uuid VARCHAR(40) PRIMARY KEY,
    group_id VARCHAR(256),
    epoch BIGINT NOT NULL DEFAULT 0,
    message_type VARCHAR(16),
    sender VARCHAR(256),
    recipient VARCHAR(256),
    payload TEXT,
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
CREATE INDEX mls_messages_recipient ON mls_messages (recipient, status);
