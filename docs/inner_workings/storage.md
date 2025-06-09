# Storage

## Overview

The infrastructure and stores the following information:

-   `Data-Set`
    -   uploaded by the Dashboard or SDK-Library
    -   metadata in database stored and binary-blob on disc
    -   2 Types: `MNIST-Data-Sets` and `Table-Data-Sets`
    -   stored data in a custom format for better access to the payload
-   `Cluster-Checkpoints`
    -   coming from Kyouko in order to backup a trained cluster
    -   metadata in database stored and binary-blob on disc
<!-- -   `Audit-Log`
    -   coming from the Torii for each incoming REST-API-request
    -   stored within the database of Shiori
-   `Error-Log`
    -   coming from each component in case of an error-message (every time when `LOG_ERROR` is
        triggered within the code)
    -   stored within the database of Shiori -->
