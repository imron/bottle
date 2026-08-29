use std::process::Command;

use tempfile::TempDir;

fn bottle() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bottle"))
}

fn run(args: &[&str]) -> (i32, String, String) {
    let out = bottle().args(args).output().expect("run bottle");
    (
        out.status.code().expect("status code"),
        String::from_utf8(out.stdout).expect("stdout utf8"),
        String::from_utf8(out.stderr).expect("stderr utf8"),
    )
}

fn run_db(dir: &TempDir, args: &[&str]) -> (i32, String, String) {
    let db = dir.path().join("bottle.db");
    let mut cmd = bottle();
    cmd.arg("--db").arg(&db).args(args);
    let out = cmd.output().expect("run bottle");
    (
        out.status.code().expect("status code"),
        String::from_utf8(out.stdout).expect("stdout utf8"),
        String::from_utf8(out.stderr).expect("stderr utf8"),
    )
}

const MEAL: &str = r#"
fields:
  - name: when
    type: enum
    required: true
    values: [breakfast, lunch]
  - name: kcal
    type: number
    required: true
"#;

#[test]
fn help_is_prose_on_stdout() {
    let (code, stdout, stderr) = run(&["help"]);
    assert_eq!(code, 0);
    assert!(stdout.starts_with("# overview\n"), "{stdout}");
    assert!(stderr.is_empty(), "{stderr}");
}

#[test]
fn help_log_and_schema_add() {
    let (code, stdout, stderr) = run(&["help", "log"]);
    assert_eq!(code, 0);
    assert!(stdout.starts_with("# log\n"), "{stdout}");
    assert!(stderr.is_empty(), "{stderr}");
    let (code, stdout, _) = run(&["help", "schema", "add"]);
    assert_eq!(code, 0);
    assert!(stdout.starts_with("# schema add\n"), "{stdout}");
}

#[test]
fn mcp_help_and_subcommand() {
    let (code, stdout, stderr) = run(&["help", "mcp"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.starts_with("# mcp\n"), "{stdout}");
    let (code, stdout, stderr) = run(&["mcp", "--help"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        stdout.contains("MCP") || stdout.contains("stdio") || stdout.contains("mcp"),
        "{stdout}{stderr}"
    );
}

#[test]
fn unknown_help_topic_is_usage() {
    let (code, stdout, stderr) = run(&["help", "nope"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("unknown help topic"), "{stderr}");
}

#[test]
fn missing_command_is_usage() {
    let (code, stdout, stderr) = run(&[]);
    assert_eq!(code, 2);
    assert!(
        stdout.is_empty() || stderr.contains("Usage"),
        "{stdout}{stderr}"
    );
    assert!(!stderr.is_empty(), "{stderr}");
    for needle in [
        "Write one entry of a registered schema",
        "List entries of a schema",
        "Declare and change types of entry",
    ] {
        assert!(stderr.contains(needle), "missing {needle:?} in {stderr}");
    }
}

fn clap_line_has_help(line: &str) -> bool {
    let rest = line.trim_start();
    match rest.rfind("  ") {
        Some(i) => rest[i..].trim().chars().any(|c| c.is_ascii_alphabetic()),
        None => false,
    }
}

fn assert_help_describes_args(args: &[&str]) {
    let (code, stdout, stderr) = run(args);
    assert_eq!(code, 0, "{args:?}: {stderr}");
    let text = if stdout.contains("Usage:") {
        stdout
    } else {
        stderr
    };
    let mut block = false;
    for line in text.lines() {
        if line == "Arguments:" || line == "Options:" {
            block = true;
            continue;
        }
        if block && (line.is_empty() || !line.starts_with(' ')) {
            block = false;
            continue;
        }
        if block {
            assert!(
                clap_line_has_help(line),
                "{args:?}: missing help on {line:?}\n{text}"
            );
        }
    }
}

#[test]
fn every_help_page_describes_args() {
    assert_help_describes_args(&["--help"]);
    assert_help_describes_args(&["help", "--help"]);
    assert_help_describes_args(&["mcp", "--help"]);
    assert_help_describes_args(&["schema", "--help"]);
    for sub in [
        "list",
        "show",
        "add",
        "add-field",
        "add-value",
        "rename",
        "rename-field",
        "retire",
        "drop",
    ] {
        assert_help_describes_args(&["schema", sub, "--help"]);
    }
    for cmd in [
        "log", "ls", "get", "sum", "last", "today", "amend", "ignore", "unignore", "backup",
    ] {
        assert_help_describes_args(&[cmd, "--help"]);
    }
}

#[test]
fn list_is_an_alias_of_ls() {
    let (code, stdout, _) = run(&["list", "--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("List entries of a schema"), "{stdout}");
    let (code, stdout, _) = run(&["schema", "ls", "--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("List registered schemas"), "{stdout}");
}

#[test]
fn schema_log_ls_through_argv() {
    let dir = TempDir::new().unwrap();
    let spec = dir.path().join("meal.yaml");
    std::fs::write(&spec, MEAL).unwrap();
    let (code, stdout, stderr) = run_db(
        &dir,
        &[
            "schema",
            "add",
            "nutrition.meal",
            "--file",
            spec.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    let (code, stdout, stderr) = run_db(&dir, &["schema", "list"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "name\tretired\nnutrition.meal\tfalse\n");
    let (code, stdout, stderr) = run_db(
        &dir,
        &[
            "log",
            "nutrition.meal",
            "--at",
            "2026-08-22T08:14:00+10:00",
            "when=breakfast",
            "kcal=568",
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.starts_with("id\tat\tlinks\n1\t"), "{stdout}");
    let (code, stdout, stderr) = run_db(&dir, &["ls", "nutrition.meal"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("breakfast"), "{stdout}");
    assert!(stdout.contains("568"), "{stdout}");
}

#[test]
fn log_file_dash_is_stdin() {
    use std::io::Write;
    use std::process::Stdio;

    let dir = TempDir::new().unwrap();
    let spec = dir.path().join("meal.yaml");
    std::fs::write(&spec, MEAL).unwrap();
    let (code, _, stderr) = run_db(
        &dir,
        &[
            "schema",
            "add",
            "nutrition.meal",
            "--file",
            spec.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    let db = dir.path().join("bottle.db");
    let mut child = bottle()
        .arg("--db")
        .arg(&db)
        .args(["log", "nutrition.meal", "--file", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn log --file -");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"when\tkcal\nbreakfast\t1\n")
        .expect("write tsv");
    let out = child.wait_with_output().expect("wait");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.starts_with("id\tat\tlinks\n1\t"), "{stdout}");
}

#[test]
fn add_field_values_trim_spaces_after_comma() {
    let dir = TempDir::new().unwrap();
    let spec = dir.path().join("meal.yaml");
    std::fs::write(&spec, MEAL).unwrap();
    run_db(
        &dir,
        &[
            "schema",
            "add",
            "nutrition.meal",
            "--file",
            spec.to_str().unwrap(),
        ],
    );
    let (code, stdout, stderr) = run_db(
        &dir,
        &[
            "schema",
            "add-field",
            "nutrition.meal",
            "--name",
            "mood",
            "--type",
            "enum",
            "--values",
            "happy, sad",
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    let (code, stdout, stderr) = run_db(&dir, &["schema", "show", "nutrition.meal"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("mood\tenum\tfalse\thappy,sad"), "{stdout}");
}

#[test]
fn date_only_at_prints_the_day() {
    let dir = TempDir::new().unwrap();
    let spec = dir.path().join("meal.yaml");
    std::fs::write(&spec, MEAL).unwrap();
    run_db(
        &dir,
        &[
            "schema",
            "add",
            "nutrition.meal",
            "--file",
            spec.to_str().unwrap(),
        ],
    );
    let (code, stdout, stderr) = run_db(
        &dir,
        &[
            "log",
            "nutrition.meal",
            "--at",
            "2026-08-22",
            "when=breakfast",
            "kcal=1",
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("2026-08-22\t"), "{stdout}");
    assert!(!stdout.contains("2026-08-22T"), "{stdout}");
}

#[test]
fn zero_id_is_usage_on_stderr() {
    let dir = TempDir::new().unwrap();
    let (code, stdout, stderr) = run_db(&dir, &["get", "meal", "0"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("invalid id"), "{stderr}");
}

#[test]
fn unknown_schema_is_fail_on_stderr() {
    let dir = TempDir::new().unwrap();
    let (code, stdout, stderr) = run_db(&dir, &["ls", "no.such"]);
    assert_eq!(code, 1);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("unknown schema"), "{stderr}");
}

#[test]
fn usage_does_not_create_db() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("bottle.db");
    let out = bottle()
        .arg("--db")
        .arg(&db)
        .args(["log", "meal", "--at", "2026-08-22T08:14:00z", "kcal=1"])
        .output()
        .expect("run bottle");
    assert_eq!(out.status.code().expect("status code"), 2);
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    assert!(stderr.contains("invalid time"), "{stderr}");
    assert!(!db.exists(), "usage must not create {}", db.display());
}
