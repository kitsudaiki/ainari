-- marks the virtual_machines, which are migrated right now. The mark is shared by all replicas of
-- hanami, so the primary key allows only one migration of a virtual_machine at the same time.
-- binary collation, so comparisons are case-sensitive like in sqlite
CREATE TABLE virtual_machine_migrations (
    virtual_machine_uuid VARCHAR(40) PRIMARY KEY,
    -- identifies the replica's mark, so it never removes the mark of another replica
    lock_uuid VARCHAR(40) NOT NULL,
    -- replica, which runs the migration
    owner VARCHAR(256) NOT NULL,
    started_at VARCHAR(64) NOT NULL,
    -- refreshed by the running migration, so the mark of a crashed replica can be detected
    heartbeat_at VARCHAR(64) NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
