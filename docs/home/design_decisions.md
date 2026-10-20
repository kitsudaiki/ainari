# Design decitions

This is a list of designe decisions, which were made on purpose.

## Sakura task abort after restart

All tasks, which were ACTIVE, QUEUED or CREATED state while the sakura instance or its host is restared,
are set into an ERROR state after the instance is online again.

The reason for this is, that persisting the task-state in the database to resume them, would require
to persist the user-token too. Wouldn't be such a big deal, because the sakura database is only
SQlite stored on the same host. But it is very unlikely, that there is more than one task open in
the task-queue per VM and resuming an active task would be difficult, because depending on the task
there are many edge cases possible. The restart shouldn't be happening in general, as long as there
are VMs on the sakura host. So to keep thing simple, in this rare error-case, all open tasks are
marked as ERROR instead of implementing a big resume-process.

## All components have always the same version

Each component within the same release also has the same version number, no matter, when the
component was introduced. So instead of starting at v0.1.0, the new component starts where the
other are already are.

This was done to make it easier to identify, which versions belong together. If each component
starts its development at v0.1.0, which others are for example at v0.11.0, v0.20.0, ... at the same
time, it becomes hard to identify, which versions belong together to have a consistent setup. When
all have always the same version, then it is clear, what belongs together.
