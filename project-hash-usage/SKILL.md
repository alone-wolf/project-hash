---
name: project-hash-usage
description: Use when users ask how to use the installed `project-hash` binary, including supported commands, how to write `project-hash.yaml`, how unit matching works, and what `--init`, `--json`, `--list-files`, and `--explain` do.
---

# project-hash Usage

This skill explains how to use `project-hash` after the binary is already installed and available in the shell as `project-hash`.

## What It Does

`project-hash` computes one stable hash for the current input files of one configured `unit`.

It is useful when someone wants a reproducible fingerprint of a selected file set.

It does not:

- store previous hashes
- decide changed vs unchanged by itself
- run builds
- watch files

## Basic Usage

If the config file is named `project-hash.yaml` and is in the current directory, `-c` can be omitted.

Hash one unit:

```bash
project-hash -u web-ui
```

Create a sample config in the current directory:

```bash
project-hash --init
```

Show help:

```bash
project-hash --help
```

Show version:

```bash
project-hash --version
```

## Supported CLI Flags

- `-c, --config <PATH>`: path to the YAML config file; defaults to `./project-hash.yaml`
- `-u, --unit <NAME>`: the unit name to hash
- `--init`: create a sample `project-hash.yaml`
- `--json`: print JSON output
- `--list-files`: print the matched file list
- `--explain`: explain how each scanned file was classified
- `-h, --help`: print help
- `-V, --version`: print version

## Flag Behavior

- `--unit` is required unless `--init` is used.
- If `--config` is omitted, the default config path is `./project-hash.yaml`.
- `--init` cannot be combined with `--config`, `--unit`, `--json`, `--list-files`, or `--explain`.
- Default mode prints only the final hash and a trailing newline.
- `--list-files` without `--json` prints only matched file paths, one per line.
- `--list-files` without `--json` prints nothing if no files matched.
- `--json` prints `unit`, `hash`, and `file_count`.
- `--json --list-files` also includes a `files` array.
- `--explain` prints a diagnostic report showing each scanned file as `included`, `excluded`, or `unmatched`.
- `--json --explain` includes the same diagnostic information in structured JSON.
- A unit with zero matched files still succeeds and returns the stable hash for an empty file set.

## Common Commands

Initialize a sample config:

```bash
project-hash --init
```

Hash one unit:

```bash
project-hash -u web-ui
```

Print JSON:

```bash
project-hash -u web-ui --json
```

List only the files used for hashing:

```bash
project-hash -u web-ui --list-files
```

Print JSON plus the file list:

```bash
project-hash -u web-ui --json --list-files
```

Explain why files were or were not included:

```bash
project-hash -u web-ui --explain
```

Use a different config path:

```bash
project-hash -c /path/to/project-hash.yaml -u api-server
```

## Makefile Template

A ready-to-copy Makefile template is available at `assets/Makefile.template`.

It shows how to:

- call `project-hash` directly from Make
- skip a build when the current unit hash matches the previous recorded hash
- inspect the current unit with `print-hash`, `list-files`, and `explain`

The main variables to customize are:

- `UNIT`: unit name from `project-hash.yaml`
- `BUILD_CMD`: the real build command to run when inputs changed
- `CONFIG`: optional config path; leave empty to use `./project-hash.yaml`
- `STATE_DIR`: where previous hashes are stored

## Config File Format

The config file is YAML with top-level `version` and `units`.

Each unit contains:

- `root`: root directory for that unit
- `include`: glob patterns to include
- `exclude`: optional glob patterns to exclude

Example:

```yaml
version: 1
units:
  web-ui:
    root: ./apps/web-ui
    include:
      - "src/**/*"
      - "package.json"
      - "pnpm-lock.yaml"
    exclude:
      - "dist/**/*"
      - "**/*.tmp"

  api-server:
    root: ./services/api
    include:
      - "src/**/*.rs"
      - "Cargo.toml"
      - "Cargo.lock"
```

## Config Rules

- If `root` is relative, it is resolved relative to the config file location.
- `include` and `exclude` patterns are matched relative to `unit.root`, not relative to the config file.

This is the most common source of confusion.

Example:

```yaml
units:
  api:
    root: ./src
    include:
      - "src/**/*.rs"
```

This usually matches nothing, because once `root` is `./src`, the pattern should usually be:

```yaml
include:
  - "**/*.rs"
```

## What Changes The Hash

The final hash changes when the selected file set changes in ways such as:

- file content changes
- file additions
- file deletions
- renames
- moves

That is because both file contents and relative paths are part of the final result.

## Output Examples

Plain hash:

```text
1f5aa6d43f2b4cc07a0df67d0620ef9fdb6fc375d38d3d0a6ff4b877f365f7af
```

JSON:

```json
{"unit":"web-ui","hash":"1f5aa6d43f2b4cc07a0df67d0620ef9fdb6fc375d38d3d0a6ff4b877f365f7af","file_count":42}
```

JSON with files:

```json
{"unit":"web-ui","hash":"1f5aa6d43f2b4cc07a0df67d0620ef9fdb6fc375d38d3d0a6ff4b877f365f7af","file_count":42,"files":["package.json","pnpm-lock.yaml","src/main.ts"]}
```

## Common Mistakes

- Forgetting to pass `--unit`
- Assuming `project-hash.yaml` will be found outside the current directory when `-c` is omitted
- Expecting `--list-files` to print the hash in plain mode
- Expecting `--list-files` to explain why files were skipped
- Writing `include` globs as config-relative instead of `unit.root`-relative
- Treating zero matched files as an execution error
