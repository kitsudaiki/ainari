-- binary collation, so comparisons and unique-checks are case-sensitive like in sqlite
CREATE TABLE user_project_mapping (
    project_id VARCHAR(256),
    user_id VARCHAR(256),
    role VARCHAR(64),
    status VARCHAR(8),
    created_at VARCHAR(64),
    created_by VARCHAR(256),
    updated_at VARCHAR(64),
    updated_by VARCHAR(256),
    deleted_at VARCHAR(64),
    deleted_by VARCHAR(256),

    -- mysql has no partial indexes, so the unique index is built on a generated column, which is
    -- NULL for deleted entries. NULL never collides within a unique index, so deleted entries
    -- don't block their pair, like the partial index of the sqlite-database.
    active_project_id VARCHAR(256)
        GENERATED ALWAYS AS (IF(status = 'ACTIVE', project_id, NULL)) VIRTUAL,

    UNIQUE INDEX user_project_mapping_active_project_user (active_project_id, user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
