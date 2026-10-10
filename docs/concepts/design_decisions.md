# Design decitions

This is a list of designe decisions, which were made on purpose.

## Sakura task abort after restart

All tasks, which were ACTIVE, QUEUED or CREATED state while the sakura instance or its host is restared, 
are set into an ERROR state after the instance is online again.

The reson for this is, that persisting the task-state in the database to resume them, would require to persist the user-token too. 
Wouldn't be such a big deal, because the sakura databse is only SQlite stored on the same host. But it is very unlikely, 
that there is more than one task open in the task-queue per VM and resuming an active task would be difficult, 
because depending on the task there are many edge cases possible. The restart shouldn't be happening in general, 
as long as there are VMs on the sakura host. So to keep thing simple, in this rare error-case, 
all open tasks are marked as ERROR instead of implementing a big resume-process.
