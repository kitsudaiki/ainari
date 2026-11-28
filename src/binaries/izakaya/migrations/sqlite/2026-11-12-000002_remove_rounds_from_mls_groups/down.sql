ALTER TABLE mls_groups ADD COLUMN round_epoch BIGINT;
ALTER TABLE mls_groups ADD COLUMN round_phase VARCHAR(16);
ALTER TABLE mls_groups ADD COLUMN round_acks TEXT;
ALTER TABLE mls_groups ADD COLUMN round_updated_at BIGINT;
