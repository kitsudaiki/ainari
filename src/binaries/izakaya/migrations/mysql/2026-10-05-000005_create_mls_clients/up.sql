-- last contact of every gateway as unix-time in seconds, which tells, if it is still alive
CREATE TABLE mls_clients (
    client_id VARCHAR(256) PRIMARY KEY NOT NULL,
    last_seen BIGINT NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
