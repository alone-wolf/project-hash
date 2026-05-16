# project-hash

`project-hash` is a stateless Rust CLI that computes a stable BLAKE3 hash for the input file set of a configured `unit`.

It is intended for scripts, CI jobs, Makefiles, and orchestrators that need a reproducible hash for the current inputs of one unit. It does not store state, compare against previous runs, decide changed vs unchanged, execute builds, or integrate with file watching or Git diff logic.

## Build and install

Build locally:

```bash
cargo build
```

Run directly with Cargo:

```bash
cargo run -- -c ./project-hash.yaml -u web-ui
```

Install the binary into your Cargo bin directory:

```bash
cargo install --path .
```

## CLI

Supported flags:

- `-c, --config <PATH>`: YAML config file path. Defaults to `./project-hash.yaml`.
- `-u, --unit <NAME>`: unit name to hash.
- `--init`: create a full sample `project-hash.yaml` in the current directory.
- `--json`: emit structured JSON.
- `--list-files`: list the files that participate in hashing.
- `--explain`: explain how each scanned file was classified.
- `-h, --help`: show help.
- `-V, --version`: show version.

If `--config` is omitted, `project-hash` reads `./project-hash.yaml`.
Default mode prints only the final hash string, followed by a newline.

Initialize a sample config:

```bash
project-hash --init
```

This writes `./project-hash.yaml` and fails if the file already exists.
`--init` cannot be combined with `--config`, `--unit`, `--json`, `--list-files`, or `--explain`.

## Configuration format

The configuration file is YAML with top-level `version` and `units` keys.

Each `unit` contains:

- `root`: root directory for that unit.
- `include`: glob patterns, interpreted relative to `root`.
- `exclude`: optional glob patterns, also relative to `root`.

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

`root` is resolved relative to the config file location when it is not absolute.

Running `project-hash --init` creates a more complete starter config with `web-ui`, `api-server`, and `docs` units that you can edit for your repository.

## Hashing behavior

`project-hash` computes the final unit hash with these rules:

1. Resolve the selected `unit`.
2. Walk the unit root directory without following symlinks.
3. Collect regular files that match `include` and do not match `exclude`.
4. Convert every matched file path to a path relative to `unit.root`.
5. Normalize path separators to `/`.
6. Sort files by relative path.
7. Hash each file's contents with BLAKE3.
8. Hash a stable internal manifest that includes both relative paths and per-file content hashes.

Because the path is part of the manifest, content changes, additions, deletions, renames, and moves all change the final hash.

If a unit matches zero files, the command still succeeds and returns a stable hash for the empty file set.

## Output examples

Normal output:

```bash
$ project-hash -u web-ui
1f5aa6d43f2b4cc07a0df67d0620ef9fdb6fc375d38d3d0a6ff4b877f365f7af
```

JSON output:

```bash
$ project-hash -u web-ui --json
{"unit":"web-ui","hash":"1f5aa6d43f2b4cc07a0df67d0620ef9fdb6fc375d38d3d0a6ff4b877f365f7af","file_count":42}
```

List files only:

```bash
$ project-hash -u web-ui --list-files
package.json
pnpm-lock.yaml
src/main.ts
src/routes/home.tsx
```

JSON with file list:

```bash
$ project-hash -u web-ui --json --list-files
{"unit":"web-ui","hash":"1f5aa6d43f2b4cc07a0df67d0620ef9fdb6fc375d38d3d0a6ff4b877f365f7af","file_count":42,"files":["package.json","pnpm-lock.yaml","src/main.ts","src/routes/home.tsx"]}
```

Explain why files were included, excluded, or unmatched:

```bash
$ project-hash -u web-ui --explain
unit: web-ui
root: /repo/apps/web-ui
hash: 1f5aa6d43f2b4cc07a0df67d0620ef9fdb6fc375d38d3d0a6ff4b877f365f7af
file_count: 2
scanned_file_count: 4
included_count: 2
excluded_count: 1
unmatched_count: 1
include_patterns:
  - src/**/*
exclude_patterns:
  - **/*.tmp
included_files:
  - src/main.ts
    status: included
    include_matches: src/**/*
    exclude_matches: none
  - src/routes/home.tsx
    status: included
    include_matches: src/**/*
    exclude_matches: none
excluded_files:
  - src/generated.tmp
    status: excluded
    include_matches: src/**/*
    exclude_matches: **/*.tmp
unmatched_files:
  - README.md
    status: unmatched
    include_matches: none
    exclude_matches: none
```

JSON with explain output:

```bash
$ project-hash -u web-ui --json --explain
{"unit":"web-ui","hash":"1f5aa6d43f2b4cc07a0df67d0620ef9fdb6fc375d38d3d0a6ff4b877f365f7af","file_count":2,"explain":{"root":"/repo/apps/web-ui","scanned_file_count":4,"included_count":2,"excluded_count":1,"unmatched_count":1,"entries":[{"path":"README.md","status":"unmatched","include_matches":[],"exclude_matches":[]},{"path":"src/generated.tmp","status":"excluded","include_matches":["src/**/*"],"exclude_matches":["**/*.tmp"]},{"path":"src/main.ts","status":"included","include_matches":["src/**/*"],"exclude_matches":[]}]}}
```

## Using it from shell scripts

`project-hash` does not save state or decide whether something changed. If you want to compare the current hash with the previous one, do it outside the tool:

```bash
#!/usr/bin/env bash
set -euo pipefail

state_file=".cache/web-ui.hash"
current_hash="$(project-hash -c ./project-hash.yaml -u web-ui)"
previous_hash=""

if [[ -f "$state_file" ]]; then
  previous_hash="$(cat "$state_file")"
fi

if [[ "$current_hash" != "$previous_hash" ]]; then
  echo "inputs changed, run build"
  mkdir -p "$(dirname "$state_file")"
  printf '%s\n' "$current_hash" > "$state_file"
else
  echo "inputs unchanged, skip build"
fi
```

That state file is owned by your script or orchestration layer, not by `project-hash`.
