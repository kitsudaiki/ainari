# Authorization

## Overview

Folling tasks:

-   manage projects
-   manage user
-   create JWT-Token in case of a login and validate them for each API-access
    -   for the token the `jwt-cpp`-library is used
    -   only Token-type `HS256` available at the moment
-   generating REST-API-Documation
    -   request all components for their REST-API-information and converts all these data into one
        file
    -   information for the documentation comes from the `libKitsunemimiSakureLang`-library of each
        component, which provides the API-endpoints

## Role-system

3 Types of users:

-   **Admin**:
    -   can see all resources of all users in all projects
    -   can manage (create, delete, ...) users and projects. This is hard defined by the code and
        can not be changed by the policy-file
    -   an Admin-user can not deleted by him/her self, so at least one Admin does every time exist
    -   the initial Admin, after a rollout of a new deployment, is created by sakura
    -   Admins have per default the role `admin` and the project-id `admin`
-   **Project-Admin**:
    -   role is bonded to a specific project
    -   can see all resources of all users within the project, where the Project-Admin-role is
        bonded to
-   **User**
    -   default
    -   role is bonded to a specific project
    -   can only see the own resources within within the project, where the User-role is bonded to

Each resource, which is created (for example a Cluster or Dataset), get the user- and project-id
attached, who created the resource. Even users and projects get the id of the admin attached, who
created the user or project
