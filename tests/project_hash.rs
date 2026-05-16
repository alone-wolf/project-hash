use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use project_hash::cli::Cli;
use project_hash::config::{
    DEFAULT_CONFIG_FILE_NAME, initialize_sample_config_in_dir, load_config, resolve_unit,
};
use project_hash::hasher::calculate_unit_hash;
use project_hash::run;
use project_hash::scanner::{explain_unit_files, scan_unit_files};
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn parses_config_file() {
    let workspace = TestWorkspace::new();
    let config_path = workspace.write_config(
        r#"
version: 1
units:
  web-ui:
    root: ./web-ui
    include:
      - "src/**/*.rs"
    exclude:
      - "**/*.tmp"
"#,
    );

    let config = load_config(&config_path).expect("config should parse");

    assert!(config.version.is_number());
    let unit = config.units.get("web-ui").expect("unit should exist");
    assert_eq!(unit.root, PathBuf::from("./web-ui"));
    assert_eq!(unit.include, vec!["src/**/*.rs"]);
    assert_eq!(unit.exclude, vec!["**/*.tmp"]);
}

#[test]
fn returns_error_when_unit_does_not_exist() {
    let workspace = TestWorkspace::new();
    let config_path = workspace.write_config(
        r#"
version: 1
units:
  web-ui:
    root: ./web-ui
    include:
      - "**/*"
"#,
    );

    let config = load_config(&config_path).expect("config should parse");
    let error = resolve_unit(&config, &config_path, "missing").expect_err("unit should be missing");

    assert!(error.to_string().contains("unit 'missing' not found"));
}

#[test]
fn include_patterns_match_expected_files() {
    let workspace = TestWorkspace::new();
    workspace.write_file("web-ui/src/main.rs", "fn main() {}\n");
    workspace.write_file("web-ui/README.md", "# docs\n");
    let config_path = workspace.write_config(
        r#"
version: 1
units:
  web-ui:
    root: ./web-ui
    include:
      - "src/**/*.rs"
"#,
    );

    let config = load_config(&config_path).expect("config should parse");
    let unit = resolve_unit(&config, &config_path, "web-ui").expect("unit should resolve");
    let files = scan_unit_files(&unit).expect("scan should succeed");

    assert_eq!(relative_paths(&files), vec!["src/main.rs"]);
}

#[test]
fn exclude_patterns_remove_matching_files() {
    let workspace = TestWorkspace::new();
    workspace.write_file("web-ui/src/main.rs", "fn main() {}\n");
    workspace.write_file("web-ui/src/generated.tmp", "ignore me\n");
    let config_path = workspace.write_config(
        r#"
version: 1
units:
  web-ui:
    root: ./web-ui
    include:
      - "src/*"
    exclude:
      - "**/*.tmp"
"#,
    );

    let config = load_config(&config_path).expect("config should parse");
    let unit = resolve_unit(&config, &config_path, "web-ui").expect("unit should resolve");
    let files = scan_unit_files(&unit).expect("scan should succeed");

    assert_eq!(relative_paths(&files), vec!["src/main.rs"]);
}

#[test]
fn repeated_hash_calculation_is_stable() {
    let workspace = TestWorkspace::new();
    workspace.write_file("web-ui/src/main.rs", "fn main() {}\n");
    workspace.write_file("web-ui/src/lib.rs", "pub fn value() -> u32 { 1 }\n");
    let config_path = workspace.write_config(
        r#"
version: 1
units:
  web-ui:
    root: ./web-ui
    include:
      - "src/**/*.rs"
"#,
    );

    let config = load_config(&config_path).expect("config should parse");
    let unit = resolve_unit(&config, &config_path, "web-ui").expect("unit should resolve");
    let files = scan_unit_files(&unit).expect("scan should succeed");

    let first = calculate_unit_hash("web-ui", &files).expect("hash should succeed");
    let second = calculate_unit_hash("web-ui", &files).expect("hash should succeed");

    assert_eq!(first.hash, second.hash);
}

#[test]
fn content_changes_affect_hash() {
    let workspace = TestWorkspace::new();
    workspace.write_file("web-ui/src/main.rs", "fn main() {}\n");
    let config_path = workspace.write_config(
        r#"
version: 1
units:
  web-ui:
    root: ./web-ui
    include:
      - "src/**/*.rs"
"#,
    );

    let config = load_config(&config_path).expect("config should parse");
    let unit = resolve_unit(&config, &config_path, "web-ui").expect("unit should resolve");
    let before = calculate_unit_hash(
        "web-ui",
        &scan_unit_files(&unit).expect("scan should succeed"),
    )
    .expect("hash should succeed");

    workspace.write_file(
        "web-ui/src/main.rs",
        "fn main() { println!(\"changed\"); }\n",
    );

    let after = calculate_unit_hash(
        "web-ui",
        &scan_unit_files(&unit).expect("scan should succeed"),
    )
    .expect("hash should succeed");

    assert_ne!(before.hash, after.hash);
}

#[test]
fn path_changes_affect_hash() {
    let workspace = TestWorkspace::new();
    workspace.write_file("web-ui/src/original.rs", "pub fn value() {}\n");
    let config_path = workspace.write_config(
        r#"
version: 1
units:
  web-ui:
    root: ./web-ui
    include:
      - "src/**/*.rs"
"#,
    );

    let config = load_config(&config_path).expect("config should parse");
    let unit = resolve_unit(&config, &config_path, "web-ui").expect("unit should resolve");
    let before = calculate_unit_hash(
        "web-ui",
        &scan_unit_files(&unit).expect("scan should succeed"),
    )
    .expect("hash should succeed");

    fs::rename(
        workspace.root().join("web-ui/src/original.rs"),
        workspace.root().join("web-ui/src/renamed.rs"),
    )
    .expect("rename should succeed");

    let after = calculate_unit_hash(
        "web-ui",
        &scan_unit_files(&unit).expect("scan should succeed"),
    )
    .expect("hash should succeed");

    assert_ne!(before.hash, after.hash);
}

#[test]
fn file_order_does_not_affect_final_hash() {
    let workspace = TestWorkspace::new();
    workspace.write_file("web-ui/src/a.rs", "pub fn a() {}\n");
    workspace.write_file("web-ui/src/b.rs", "pub fn b() {}\n");
    let config_path = workspace.write_config(
        r#"
version: 1
units:
  web-ui:
    root: ./web-ui
    include:
      - "src/**/*.rs"
"#,
    );

    let config = load_config(&config_path).expect("config should parse");
    let unit = resolve_unit(&config, &config_path, "web-ui").expect("unit should resolve");
    let files = scan_unit_files(&unit).expect("scan should succeed");
    let mut reversed = files.clone();
    reversed.reverse();

    let ordered_hash = calculate_unit_hash("web-ui", &files).expect("hash should succeed");
    let reversed_hash = calculate_unit_hash("web-ui", &reversed).expect("hash should succeed");

    assert_eq!(ordered_hash.hash, reversed_hash.hash);
}

#[test]
fn json_output_contains_unit_hash_and_file_count() {
    let workspace = TestWorkspace::new();
    workspace.write_file("web-ui/src/main.rs", "fn main() {}\n");
    let config_path = workspace.write_config(
        r#"
version: 1
units:
  web-ui:
    root: ./web-ui
    include:
      - "src/**/*.rs"
"#,
    );

    let output = run(Cli {
        config: Some(config_path),
        unit: Some("web-ui".to_owned()),
        init: false,
        json: true,
        list_files: true,
        explain: false,
    })
    .expect("run should succeed");

    let parsed: Value = serde_json::from_str(&output).expect("output should be valid json");

    assert_eq!(parsed["unit"], "web-ui");
    assert_eq!(parsed["file_count"], 1);
    assert!(parsed["hash"].as_str().is_some());
    assert_eq!(parsed["files"], serde_json::json!(["src/main.rs"]));
}

#[test]
fn cli_allows_omitting_config_when_unit_is_present() {
    let cli = Cli::try_parse_from(["project-hash", "-u", "web-ui"]).expect("cli should parse");

    assert_eq!(cli.config, None);
    assert_eq!(cli.unit.as_deref(), Some("web-ui"));
}

#[test]
fn binary_uses_default_config_file_when_config_flag_is_omitted() {
    let workspace = TestWorkspace::new();
    workspace.write_file("web-ui/src/main.rs", "fn main() {}\n");
    let config_path = workspace.write_config(
        r#"
version: 1
units:
  web-ui:
    root: ./web-ui
    include:
      - "src/**/*.rs"
"#,
    );

    let config = load_config(&config_path).expect("config should parse");
    let unit = resolve_unit(&config, &config_path, "web-ui").expect("unit should resolve");
    let files = scan_unit_files(&unit).expect("scan should succeed");
    let expected = calculate_unit_hash("web-ui", &files).expect("hash should succeed");

    let output = Command::new(env!("CARGO_BIN_EXE_project-hash"))
        .current_dir(workspace.root())
        .arg("-u")
        .arg("web-ui")
        .output()
        .expect("binary should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf-8"),
        format!("{}\n", expected.hash)
    );
}

#[test]
fn explain_output_shows_included_excluded_and_unmatched_files() {
    let workspace = TestWorkspace::new();
    workspace.write_file("web-ui/src/main.rs", "fn main() {}\n");
    workspace.write_file("web-ui/src/generated.tmp", "ignore me\n");
    workspace.write_file("web-ui/README.md", "# docs\n");
    let config_path = workspace.write_config(
        r#"
version: 1
units:
  web-ui:
    root: ./web-ui
    include:
      - "src/*"
      - "*.md"
    exclude:
      - "**/*.tmp"
      - "README.md"
"#,
    );

    let output = run(Cli {
        config: Some(config_path),
        unit: Some("web-ui".to_owned()),
        init: false,
        json: false,
        list_files: false,
        explain: true,
    })
    .expect("run should succeed");

    assert!(output.contains("unit: web-ui"));
    assert!(output.contains("file_count: 1"));
    assert!(output.contains("included_count: 1"));
    assert!(output.contains("excluded_count: 2"));
    assert!(output.contains("unmatched_count: 0"));
    assert!(output.contains("  - README.md"));
    assert!(output.contains("status: excluded"));
    assert!(output.contains("include_matches: *.md"));
    assert!(output.contains("exclude_matches: README.md"));
    assert!(output.contains("  - src/generated.tmp"));
    assert!(output.contains("exclude_matches: **/*.tmp"));
    assert!(output.contains("  - src/main.rs"));
    assert!(output.contains("status: included"));
}

#[test]
fn json_explain_output_contains_scan_details() {
    let workspace = TestWorkspace::new();
    workspace.write_file("web-ui/src/main.rs", "fn main() {}\n");
    workspace.write_file("web-ui/README.md", "# docs\n");
    let config_path = workspace.write_config(
        r#"
version: 1
units:
  web-ui:
    root: ./web-ui
    include:
      - "src/**/*.rs"
"#,
    );

    let output = run(Cli {
        config: Some(config_path),
        unit: Some("web-ui".to_owned()),
        init: false,
        json: true,
        list_files: true,
        explain: true,
    })
    .expect("run should succeed");

    let parsed: Value = serde_json::from_str(&output).expect("output should be valid json");
    let entries = parsed["explain"]["entries"]
        .as_array()
        .expect("entries should be an array");

    assert_eq!(parsed["files"], serde_json::json!(["src/main.rs"]));
    assert_eq!(parsed["explain"]["included_count"], 1);
    assert_eq!(parsed["explain"]["excluded_count"], 0);
    assert_eq!(parsed["explain"]["unmatched_count"], 1);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["path"], "README.md");
    assert_eq!(entries[0]["status"], "unmatched");
    assert_eq!(entries[1]["path"], "src/main.rs");
    assert_eq!(entries[1]["status"], "included");
}

#[test]
fn explain_scan_reports_root_relative_matching() {
    let workspace = TestWorkspace::new();
    workspace.write_file("src/main.rs", "fn main() {}\n");
    let config_path = workspace.write_config(
        r#"
version: 1
units:
  main:
    root: ./src
    include:
      - "src/**/*.rs"
"#,
    );

    let config = load_config(&config_path).expect("config should parse");
    let unit = resolve_unit(&config, &config_path, "main").expect("unit should resolve");
    let explanation = explain_unit_files(&unit).expect("scan should succeed");

    assert_eq!(explanation.included_count(), 0);
    assert_eq!(explanation.unmatched_count(), 1);
    assert_eq!(explanation.entries[0].relative_path, "main.rs");
    assert_eq!(explanation.entries[0].status.as_str(), "unmatched");
    assert_eq!(explanation.entries[0].include_matches, Vec::<String>::new());
}

#[test]
fn init_conflicts_with_explain_flag() {
    let error = Cli::try_parse_from(["project-hash", "--init", "--explain"])
        .expect_err("clap should reject conflicting flags");

    assert!(error.to_string().contains("--explain"));
}

#[test]
fn init_creates_sample_config_in_target_directory() {
    let workspace = TestWorkspace::new();

    let created_path =
        initialize_sample_config_in_dir(workspace.root()).expect("init should create config");
    let created_contents =
        fs::read_to_string(&created_path).expect("created config should be readable");
    let config = load_config(&created_path).expect("created config should parse");

    assert_eq!(
        created_path,
        workspace.root().join(DEFAULT_CONFIG_FILE_NAME)
    );
    assert!(created_contents.contains("web-ui"));
    assert!(created_contents.contains("api-server"));
    assert!(created_contents.contains("docs"));
    assert!(config.units.contains_key("web-ui"));
    assert!(config.units.contains_key("api-server"));
    assert!(config.units.contains_key("docs"));
}

#[test]
fn init_fails_when_config_file_already_exists() {
    let workspace = TestWorkspace::new();
    workspace.write_config(
        r#"
version: 1
units: {}
"#,
    );

    let error =
        initialize_sample_config_in_dir(workspace.root()).expect_err("init should not overwrite");

    assert!(error.to_string().contains("config file already exists"));
}

fn relative_paths(files: &[project_hash::scanner::UnitFile]) -> Vec<&str> {
    files
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect()
}

struct TestWorkspace {
    temp_dir: TempDir,
}

impl TestWorkspace {
    fn new() -> Self {
        Self {
            temp_dir: TempDir::new().expect("temp dir should be created"),
        }
    }

    fn root(&self) -> &Path {
        self.temp_dir.path()
    }

    fn write_file(&self, relative_path: &str, contents: &str) {
        let path = self.root().join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directories should be created");
        }
        fs::write(path, contents).expect("file should be written");
    }

    fn write_config(&self, contents: &str) -> PathBuf {
        let path = self.root().join("project-hash.yaml");
        fs::write(&path, contents.trim_start()).expect("config should be written");
        path
    }
}
