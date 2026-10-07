-- an isolated host is only used by the virtual-machines of a single project
ALTER TABLE hosts ADD COLUMN is_host_isolated BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE hosts ADD COLUMN project_id VARCHAR(256);
