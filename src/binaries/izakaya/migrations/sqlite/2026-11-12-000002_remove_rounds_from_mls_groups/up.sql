-- the gateways roll out the keys of a new epoch themselves, so izakaya has no rounds anymore
ALTER TABLE mls_groups DROP COLUMN round_epoch;
ALTER TABLE mls_groups DROP COLUMN round_phase;
ALTER TABLE mls_groups DROP COLUMN round_acks;
ALTER TABLE mls_groups DROP COLUMN round_updated_at;
