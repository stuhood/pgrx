# Feature Request: Fragment-based extension migration & upgrade management for development and release workflows

### Summary

We would like to propose adding support in `cargo-pgrx` for managing **unreleased SQL migration fragments**.

In active multi-contributor extension repositories, multiple branches/PRs concurrently introduce schema modifications (new SQL functions, types, operators, catalog adjustments, or migration DDL). Having every PR edit a monolithic `extension--<prev>--<target>.sql` causes continuous merge conflicts and upgrade sequencing problems.

A fragment-based workflow avoids these merge conflicts by allowing each PR to add a dedicated migration fragment (e.g. `sql/unreleased/<PR>.<description>.sql`). At release time, fragments are topologically sorted and assembled into the versioned upgrade script. During local development, `cargo pgrx install` (or a dedicated flag) can automatically assemble and deploy an ephemeral upgrade path into `$SHAREDIR/extension/` so that existing test and benchmark databases can be upgraded with `ALTER EXTENSION ... UPDATE;` without data loss or reindexing.

---

### Motivation & Problem Description

Currently, `cargo-pgrx` handles extension installation in [cargo-pgrx/src/command/install.rs](https://github.com/pgcentralfoundation/pgrx/blob/3621b1f649e75f6bad06923317a0ced3175ae089/cargo-pgrx/src/command/install.rs#L418-L439) by:
1. Generating the full base schema `extension--<version>.sql` and placing it in `$SHAREDIR/extension/`.
2. Copying any pre-existing version upgrade files matching `^extension--.+--.+\.sql$` from `sql/` into `$SHAREDIR/extension/`.

However, during active development:
1. **No Upgrade Path for In-Flight Changes**:
   When developers pull branches that introduce new native functions or SQL entities (e.g., helper functions, opclasses, or custom DDL that cannot be represented purely via `#[pg_extern]`), running `cargo pgrx install` updates the shared library (`.dylib`/`.so`) in `$PKGLIBDIR`, but **no upgrade path exists** for existing databases that already have the extension installed at the current workspace version or a previous release version.
2. **Catalog vs. Binary Skew**:
   The new shared library code expects certain functions or catalog entries to exist, but the database catalog still has the old schema. Running `ALTER EXTENSION ... UPDATE;` is a no-op because no upgrade script between the installed version and target version is present in `$SHAREDIR/extension/`.
3. **Destructive Workaround**:
   Developers are currently forced to drop and recreate the extension (`DROP EXTENSION ... CASCADE; CREATE EXTENSION ...;`). For extensions backed by large indexed datasets (such as BM25 search indexes, vector indices, or benchmark suites), recreating the extension drops all dependent indexes and requires reindexing gigabytes of data from scratch.
4. **Merge Conflict Bottleneck on Monolithic Upgrade Scripts**:
   If PRs instead commit directly to a single `extension--0.25.0--0.26.0.sql` file, every PR touching SQL conflicts with every other branch.

---

### The Fragment-Based Workflow

To solve this, we use a fragment-based workflow in ParadeDB (as described in our `RELEASE.md`):

1. **Unreleased Fragments Directory**:
   PRs that introduce DDL/schema modifications add a standalone migration fragment file to:
   ```
   <crate>/sql/unreleased/<PR_NUMBER>.<description>.sql
   ```
   *(e.g. `sql/unreleased/6221.inline_row_evaluation.sql`)*

2. **Explicit Dependency Tracking**:
   If a fragment depends on SQL objects created by another in-flight fragment, it declares dependencies in a header comment:
   ```sql
   -- depends-on: 6099, 6120
   ```
   This allows fragments to be topologically sorted into a valid execution order.

3. **Release Assembly**:
   When cutting a release, a release tool topologically sorts all fragments in `sql/unreleased/*.sql`, concatenates them into `sql/extension--<prev_version>--<target_version>.sql`, and removes the consumed fragments from the branch.

4. **Ephemeral Upgrade Path for Development (`pgrx install`)**:
   During local development, `cargo pgrx install` assembles unreleased fragments into an ephemeral upgrade script directly inside Postgres's `$SHAREDIR/extension/` (e.g. `extension--<prev>--<target>.sql`), while updating `default_version` in `$SHAREDIR/extension/extension.control`.

   Because the upgrade file is placed directly into Postgres's `$SHAREDIR` rather than the git working copy:
   - Git status stays completely clean (no dirty/untracked files in the repo).
   - `cargo pgrx install` does not need to connect to or interact with a running Postgres daemon.
   - It composes seamlessly with standard restart patterns:
     ```bash
     cargo pgrx stop && cargo pgrx install && cargo pgrx start
     ```
   - Running `ALTER EXTENSION <ext> UPDATE;` immediately upgrades the active database catalog without data loss or index rebuilding.

---

### Proposed Capabilities for `pgrx`

We would love to discuss upstreaming first-class support for this workflow into `cargo-pgrx`:

1. **First-class Unreleased Fragments Directory**:
   Recognize a standard directory convention, such as `<package>/sql/unreleased/*.sql` (configurable in `Cargo.toml` under `[package.metadata.pgrx]`).

2. **Dependency Resolution**:
   Support `-- depends-on: ...` comment annotations in fragments, with built-in topological sorting and cycle detection.

3. **Development Assembly in `cargo pgrx install`**:
   During `cargo pgrx install`, if `sql/unreleased/` contains fragments:
   - Identify the previous released version from existing `extension--*--*.sql` scripts or git tags.
   - Assemble an upgrade script directly into `dest_dir` (`$SHAREDIR/extension/extension--<prev>--<target_dev>.sql`).
   - Optionally update `default_version` in the installed `.control` file so `ALTER EXTENSION ... UPDATE;` targets it automatically.

4. **`cargo pgrx migrate` / `cargo pgrx schema assemble` Subcommand**:
   Provide a command to assemble fragments into a formal versioned upgrade script at release time:
   ```bash
   cargo pgrx schema assemble --prev-version 0.25.9 0.26.0
   ```
   with flags to either `--consume` (delete fragments) or `--preserve-fragments`.

---

### Example Reference Implementation

For reference, our current development wrapper script does the following:
1. Runs `cargo pgrx install "$@"`.
2. Checks for `sql/unreleased/*.sql`.
3. Resolves the latest target version in `sql/extension--*--*.sql`.
4. Assembles fragments in topological order into `${SHAREDIR}/extension/extension--${PREV}--${TARGET}.sql`.
5. Updates `default_version` in `${SHAREDIR}/extension/extension.control`.
