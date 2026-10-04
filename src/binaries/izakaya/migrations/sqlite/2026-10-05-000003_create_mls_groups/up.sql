-- state of the coordination of every group: its members, its committer and its key-rotation.
-- The timestamps are unix-times in seconds.
CREATE TABLE mls_groups (
    vni INTEGER PRIMARY KEY NOT NULL,
    committer VARCHAR(256) NOT NULL,
    epoch BIGINT NOT NULL,
    members TEXT NOT NULL,
    round_epoch BIGINT,
    round_phase VARCHAR(16),
    round_acks TEXT,
    round_updated_at BIGINT,
    rotated_at BIGINT NOT NULL,
    created_at BIGINT NOT NULL
);
