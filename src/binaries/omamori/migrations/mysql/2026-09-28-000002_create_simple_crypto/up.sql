-- binary collation, so comparisons and unique-checks are case-sensitive like in sqlite
CREATE TABLE simple_crypto (
    secret_uuid VARCHAR(40) PRIMARY KEY,
    encrypted_secret LONGTEXT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
