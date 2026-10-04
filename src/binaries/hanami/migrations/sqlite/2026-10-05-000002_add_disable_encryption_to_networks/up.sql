-- a network with encryption disabled gets no IPsec-encryption between its hosts and no MLS-group
ALTER TABLE networks ADD COLUMN disable_encryption BOOLEAN NOT NULL DEFAULT FALSE;
