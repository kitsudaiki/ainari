ALTER TABLE mls_groups
    ADD COLUMN round_epoch BIGINT,
    ADD COLUMN round_phase VARCHAR(16),
    ADD COLUMN round_acks TEXT,
    ADD COLUMN round_updated_at BIGINT;
