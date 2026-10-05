-- owned_by names the kind of owner of the secret and resource_uuid the owning resource, if the
-- secret doesn't belong to a user; existing secrets were all created by users
ALTER TABLE secrets
    ADD COLUMN owned_by VARCHAR(64) NOT NULL DEFAULT 'user' AFTER name,
    ADD COLUMN resource_uuid VARCHAR(40) NULL DEFAULT NULL AFTER owned_by;
