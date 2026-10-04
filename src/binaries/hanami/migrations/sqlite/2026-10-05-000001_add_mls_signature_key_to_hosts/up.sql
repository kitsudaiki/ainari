-- MLS signature-key of the torii of a sakura-host, which hanami pinned with the first membership-
-- grant of the host. Every following grant names this key, so no other torii can join a group in
-- the name of the host.
ALTER TABLE hosts ADD COLUMN mls_signature_key VARCHAR(256);
