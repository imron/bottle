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
fn date_only_at_is_usage_on_stderr() {
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
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("date-only"), "{stderr}");
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
        .args(["log", "meal", "--at", "2026-08-22", "kcal=1"])
        .output()
        .expect("run bottle");
    assert_eq!(out.status.code().expect("status code"), 2);
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    assert!(stderr.contains("date-only"), "{stderr}");
    assert!(!db.exists(), "usage must not create {}", db.display());
}
