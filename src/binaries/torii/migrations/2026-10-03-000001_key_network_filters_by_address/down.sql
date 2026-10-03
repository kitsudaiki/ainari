DROP TABLE network_filters;
CREATE TABLE network_filters (
    uuid VARCHAR(40) PRIMARY KEY,
    route_uuid VARCHAR(40),
    rule_type VARCHAR(16),
    spec VARCHAR(64),
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
