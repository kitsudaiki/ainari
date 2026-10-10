-- entries are only added and never changed or deleted, so there are no update- or delete-columns
-- created_at is an RFC-3339 timestamp in UTC, whose string-order is the chronological order
CREATE TABLE audit_logs (
    uuid VARCHAR(40) PRIMARY KEY,
    request_type VARCHAR(64) NOT NULL,
    endpoint VARCHAR(1024) NOT NULL,
    component_name VARCHAR(64) NOT NULL,
    host_name VARCHAR(256) NOT NULL,
    user_id VARCHAR(256) NOT NULL,
    project_id VARCHAR(256) NOT NULL,
    created_at VARCHAR(64) NOT NULL
);
CREATE INDEX audit_logs_created_at_idx ON audit_logs (created_at);
