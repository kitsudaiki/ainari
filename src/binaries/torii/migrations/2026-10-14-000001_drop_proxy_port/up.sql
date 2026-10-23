-- all proxies share one listener and are picked by the server name of the TLS-connection, which
-- contains the uuid of the proxy, so a proxy has no port of its own anymore
ALTER TABLE proxys DROP COLUMN port;
