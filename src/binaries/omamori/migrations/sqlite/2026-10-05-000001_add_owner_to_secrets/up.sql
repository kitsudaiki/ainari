-- owned_by names the kind of owner of the secret and resource_uuid the owning resource, if the
-- secret doesn't belong to a user; existing secrets were all created by users
-- sqlite can only append columns, so the table is rebuilt to place them behind the name-column
CREATE TABLE secrets_new (
    uuid VARCHAR(40) PRIMARY KEY,
    name VARCHAR(256),
    owned_by VARCHAR(64) NOT NULL DEFAULT 'user',
    resource_uuid VARCHAR(40) DEFAULT NULL,
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
INSERT INTO secrets_new (uuid, name, owned_by, resource_uuid, owner_id, project_id, status,
                         created_at, created_by, updated_at, updated_by, deleted_at, deleted_by)
    SELECT uuid, name, 'user', NULL, owner_id, project_id, status,
           created_at, created_by, updated_at, updated_by, deleted_at, deleted_by
    FROM secrets;
DROP TABLE secrets;
ALTER TABLE secrets_new RENAME TO secrets;
