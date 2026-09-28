-- address of the external api of a sakura-host, which is the target of its proxies
ALTER TABLE hosts ADD COLUMN external_address VARCHAR(256);
