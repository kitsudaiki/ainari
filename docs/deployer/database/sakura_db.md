# Sakura

## models

| field      | type         | is primary |
| ---------- | ------------ | ---------- |
| uuid       | VARCHAR(40)  | x          |
| name       | VARCHAR(256) |            |
| template   | TEXT         |            |
| owner_id   | VARCHAR(256) |            |
| project_id | VARCHAR(256) |            |
| status     | VARCHAR(8)   |            |
| created_at | VARCHAR(64)  |            |
| created_by | VARCHAR(256) |            |
| updated_at | VARCHAR(64)  |            |
| updated_by | VARCHAR(256) |            |
| deleted_at | VARCHAR(64)  |            |
| deleted_by | VARCHAR(256) |            |

## tasks

| field                  | type         | is primary |
| ---------------------- | ------------ | ---------- |
| uuid                   | VARCHAR(40)  | X          |
| name                   | VARCHAR(256) |            |
| model_uuid           | VARCHAR(40)  |            |
| task_type              | VARCHAR(32)  |            |
| task_state             | VARCHAR(32)  |            |
| total_number_of_epochs | INTEGER      |            |
| current_epoch          | INTEGER      |            |
| total_number_of_cycles | INTEGER      |            |
| current_cycle          | INTEGER      |            |
| queued_at              | VARCHAR(64)  |            |
| started_at             | VARCHAR(64)  |            |
| aborted_at             | VARCHAR(64)  |            |
| finished_at            | VARCHAR(64)  |            |
| error_message          | TEXT         |            |
| owner_id               | VARCHAR(256) |            |
| project_id             | VARCHAR(256) |            |
| created_at             | VARCHAR(64)  |            |
| created_by             | VARCHAR(256) |            |
