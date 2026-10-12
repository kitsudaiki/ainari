-- argon2-hashes in the PHC string format are longer than the 64 characters of the old sha256-hashes.
-- Not required for sqlite, because sqlite doesn't enforce the length of a VARCHAR.
ALTER TABLE users MODIFY pw_hash VARCHAR(256);
