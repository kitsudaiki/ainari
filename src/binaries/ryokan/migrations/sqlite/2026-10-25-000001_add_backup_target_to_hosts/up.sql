-- marks an onsen-host, which only stores backups and never normal images or snapshots. All
-- existing hosts store images, so they are no backup-targets.
ALTER TABLE hosts ADD COLUMN is_backup_target BOOLEAN NOT NULL DEFAULT FALSE;
