-- the gateways roll out the keys of a new epoch themselves, so izakaya has no rounds anymore
ALTER TABLE mls_groups
    DROP COLUMN round_epoch,
    DROP COLUMN round_phase,
    DROP COLUMN round_acks,
    DROP COLUMN round_updated_at;
