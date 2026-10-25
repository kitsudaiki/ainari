-- lock of a project during a backup or restore. The project is unlocked, while locked_by_job is
-- NULL. All existing projects are unlocked. Sqlite can only append new columns at the end of the
-- table.
ALTER TABLE projects ADD COLUMN locked_by_job VARCHAR(40) DEFAULT NULL;
ALTER TABLE projects ADD COLUMN lock_reason TEXT DEFAULT NULL;
ALTER TABLE projects ADD COLUMN locked_at VARCHAR(64) DEFAULT NULL;
