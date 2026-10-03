-- packet-filters are identified by the address of a virtual machine within its tenant and the
-- direction of the traffic instead of the uuid of a route
DROP TABLE network_filters;
CREATE TABLE network_filters (
    uuid VARCHAR(40) PRIMARY KEY,
    vni INTEGER,
    ip VARCHAR(16),
    direction VARCHAR(8),
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
