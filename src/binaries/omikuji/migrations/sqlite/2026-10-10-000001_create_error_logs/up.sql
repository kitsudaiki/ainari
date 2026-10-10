-- entries are only added and never changed or deleted, so there are no update- or delete-columns
-- created_at is an RFC-3339 timestamp in UTC, whose string-order is the chronological order
CREATE TABLE error_logs (
    uuid VARCHAR(40) PRIMARY KEY,
    message TEXT NOT NULL,
    component_name VARCHAR(64) NOT NULL,
    host_name VARCHAR(256) NOT NULL,
    user_id VARCHAR(256) NOT NULL,
    project_id VARCHAR(256) NOT NULL,
    created_at VARCHAR(64) NOT NULL
);
CREATE INDEX error_logs_created_at_idx ON error_logs (created_at);
