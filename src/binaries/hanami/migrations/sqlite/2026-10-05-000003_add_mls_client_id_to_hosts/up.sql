-- MLS-identity of the torii of a sakura-host, which hanami pinned together with its signature-key.
-- It is the underlay-address of the torii, so a torii, which comes back with another address after
-- a restart, is recognised and its old identity removed from the groups.
ALTER TABLE hosts ADD COLUMN mls_client_id VARCHAR(256);
