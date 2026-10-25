-- new UUIDs of the resources, which were restored under another UUID, because their original
-- UUID was already used
-- binary collation, so comparisons and unique-checks are case-sensitive like in sqlite
CREATE TABLE id_maps (
    job_uuid VARCHAR(40) NOT NULL,
    resource_type VARCHAR(32) NOT NULL,
    old_uuid VARCHAR(40) NOT NULL,
    new_uuid VARCHAR(40) NOT NULL,
    PRIMARY KEY (job_uuid, old_uuid)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
