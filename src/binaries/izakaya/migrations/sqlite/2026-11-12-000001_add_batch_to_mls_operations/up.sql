-- the operations, which a committer makes with one commit, share the uuid of their batch
ALTER TABLE mls_operations ADD COLUMN batch_uuid VARCHAR(40);
