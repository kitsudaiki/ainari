-- queue of the changes of the groups, which their committers make one after another. The
-- timestamps are unix-times in milliseconds, so the operations of one second keep their order.
CREATE TABLE mls_operations (
    uuid VARCHAR(40) PRIMARY KEY,
    vni INTEGER NOT NULL,
    kind VARCHAR(16) NOT NULL,
    client_id VARCHAR(256),
    grant_json LONGTEXT,
    status VARCHAR(16) NOT NULL,
    retries INTEGER NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL,
    started_at BIGINT,
    INDEX mls_operations_vni_status (vni, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
