-- membership-grants of hanami, at most one per gateway and network
CREATE TABLE mls_grants (
    uuid VARCHAR(40) PRIMARY KEY,
    vni INTEGER NOT NULL,
    client_id VARCHAR(256) NOT NULL,
    signature_key VARCHAR(256) NOT NULL,
    payload TEXT NOT NULL,
    signature VARCHAR(256) NOT NULL,
    expires_at BIGINT NOT NULL,
    created_at VARCHAR(64) NOT NULL
);
CREATE UNIQUE INDEX mls_grants_vni_client ON mls_grants (vni, client_id);
