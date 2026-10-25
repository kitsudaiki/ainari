-- new UUIDs of the resources, which were restored under another UUID, because their original
-- UUID was already used
CREATE TABLE id_maps (
    job_uuid VARCHAR(40) NOT NULL,
    resource_type VARCHAR(32) NOT NULL,
    old_uuid VARCHAR(40) NOT NULL,
    new_uuid VARCHAR(40) NOT NULL,
    PRIMARY KEY (job_uuid, old_uuid)
);
