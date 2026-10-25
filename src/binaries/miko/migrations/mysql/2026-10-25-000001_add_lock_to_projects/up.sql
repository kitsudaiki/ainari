-- lock of a project during a backup or restore. The project is unlocked, while locked_by_job is
-- NULL. All existing projects are unlocked.
ALTER TABLE projects
    ADD COLUMN locked_by_job VARCHAR(40) NULL DEFAULT NULL AFTER status,
    ADD COLUMN lock_reason LONGTEXT NULL DEFAULT NULL AFTER locked_by_job,
    ADD COLUMN locked_at VARCHAR(64) NULL DEFAULT NULL AFTER lock_reason;
