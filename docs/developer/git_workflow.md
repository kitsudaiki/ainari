# Git-Workflow

## Basic git workflow

```mermaid
%%{init: {"gitGraph": {"mainBranchName": "develop", "mainBranchOrder": 2}, "themeVariables": {"git0": "#86cfff", "gitBranchLabel0": "#000000"}}}%%
gitGraph
    commit id: " "
    branch "qa/..." order: 4
    commit id: "chore(#...): ..."
    checkout develop
    merge "qa/..." id: "pull request (qa)"
    branch "feat/..." order: 3
    commit id: "feat(#...): ..."
    checkout develop
    merge "feat/..." id: "pull request (feature)"
    branch "vx.y.z" order: 0
    commit id: "fork develop to new release"
    checkout develop
    branch "fix/..." order: 5
    commit id: "fix(#...): ..."
    checkout "vx.y.z"
    branch "tag-x.y.z" order: 1
    commit id: "tag x.y.0"
    checkout develop
    merge "fix/..." id: "pull request (fix)"
    checkout "vx.y.z"
    merge "tag-x.y.z" id: "release x.y.0"
    checkout "tag-x.y.z"
    merge develop id: "tag x.y.1 (merge or cherry-pick)"
    checkout "vx.y.z"
    merge "tag-x.y.z" id: "release x.y.1"
    checkout develop
    commit id: "  "
```

## Branches

### `develop`

- main-branch
- doesn't allow direct commits. Everything has to be added by pull requests.

### `tag-x.y.z`

- tag-branch
- for example: `tag-0.3.1`
- used for tagging new versions

### `vx.y.z`

- contains all realeases of a specific minor version
- for example: `v0.3.x` and contains versions `v0.3.0`, `v0.3.1` and so on

### `fix/...`

- bug-fix branches
- created for fixing bug-report issues
- for example: `fix/fix-creashes-in-api`

### `feat/...`

- features branches
- for new features or improvement of features
- created for feature issues
- for example: `feature/add-cuda-support`

### `qa/...`

- quality assurance branches
- for everything else: typo fixes, updating comments, fixes in documentation, updates at the
    ci-pipeline, and so on
- created for qa issues
- for example: `qa/reduce-storage-consumption-of-ci-pipeline`

## Commits

### Keywords

All the standard keywords:

- `build`
- `chore`
- `ci`
- `docs`
- `feat`
- `fix`
- `perf`
- `refactor`
- `revert`
- `style`
- `test`

### Commit-Message

All commit-messages have to match the following pattern:

```text
<KEYWORD> (#<ISSUE_NUMBER>): <SHORT_DESCRIPTION>

<LONG DESCRIPTION>
```

!!! example

    ```
    Fix (#168): removed old checkpoint endpoints

    The endpoints to create and finalize checkpoints
    were deprecated since removing the microservice-
    architecture. They were also not used interenally
    anymore. So they were removed from the code.
    ```
