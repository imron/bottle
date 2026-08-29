use std::path::PathBuf;

use bottle::{Bottle, Cmd, EntryId, Error, LogInput, cmd};

pub fn eid(n: i64) -> EntryId {
    EntryId::parse(n).unwrap()
}
use tempfile::TempDir;

pub const TZ: &str = "Australia/Melbourne";

pub struct Harness {
    pub dir: TempDir,
    bottle: Bottle,
}

pub fn harness() -> Harness {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("bottle.db");
    let bottle = Bottle::open(&db, Some("test".into()), Some(TZ)).unwrap();
    Harness { dir, bottle }
}

impl Harness {
    pub fn run(&mut self, cmd: Cmd) -> Result<String, Error> {
        let style = bottle::style(&cmd);
        let request = bottle::parse(cmd, self.bottle.tz())?;
        bottle::execute(&mut self.bottle, request, style)
    }

    pub fn run_ok(&mut self, cmd: Cmd) -> String {
        self.run(cmd).unwrap_or_else(|e| panic!("{e}"))
    }

    pub fn log_entries(&mut self, logs: Vec<cmd::Log>) -> Result<String, Error> {
        let request = bottle::logs(
            logs.into_iter().map(LogInput::from).collect(),
            self.bottle.tz(),
        )?;
        bottle::execute(&mut self.bottle, request, bottle::Style::Tsv)
    }

    pub fn yaml_file(&self, name: &str, body: &str) -> PathBuf {
        let path = self.dir.path().join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    pub fn add_schema(&mut self, name: &str, yaml: &str) {
        let file = self.yaml_file(&format!("{name}.yaml"), yaml);
        self.run_ok(Cmd::Schema(cmd::SchemaCmd::Add(cmd::SchemaAdd {
            name: name.into(),
            file,
        })));
    }

    pub fn log(
        &mut self,
        schema: &str,
        fields: &[(&str, &str)],
        links: &[(&str, &str)],
        at: Option<&str>,
    ) -> String {
        self.run_ok(Cmd::Log(cmd::Log {
            schema: schema.into(),
            at: at.map(str::to_string),
            agent: None,
            links: links
                .iter()
                .map(|(n, t)| ((*n).to_string(), (*t).to_string()))
                .collect(),
            fields: fields
                .iter()
                .map(|(n, v)| ((*n).to_string(), (*v).to_string()))
                .collect(),
        }))
    }
}

pub fn seed_meals(h: &mut Harness) {
    h.add_schema("nutrition.meal", MEAL);
    h.log(
        "nutrition.meal",
        &[
            ("when", "breakfast"),
            ("what", "eggs"),
            ("kcal", "568"),
            ("protein", "49"),
            ("carbs", "5"),
            ("fat", "39.6"),
        ],
        &[],
        Some("2026-08-22T08:14:00Z"),
    );
    h.log(
        "nutrition.meal",
        &[
            ("when", "lunch"),
            ("what", "rice"),
            ("kcal", "200"),
            ("protein", "10"),
            ("carbs", "40"),
        ],
        &[],
        Some("2026-08-23T12:00:00Z"),
    );
}

pub const MEAL: &str = r#"
fields:
  - name: when
    type: enum
    required: true
    values: [breakfast, snack, lunch, dinner, extra]
  - name: what
    type: text
    required: true
  - name: kcal
    type: number
    required: true
  - name: protein
    type: number
    required: true
  - name: carbs
    type: number
    required: true
  - name: fat
    type: number
    required: false
"#;

pub const SESSION: &str = r#"
fields:
  - name: title
    type: text
    required: false
"#;

pub const SET: &str = r#"
fields:
  - name: movement
    type: text
    required: true
  - name: reps
    type: number
    required: true
  - name: load
    type: number
    required: false
"#;

pub fn tsv_lines(s: &str) -> Vec<Vec<&str>> {
    s.trim_end_matches('\n')
        .lines()
        .map(|line| line.split('\t').collect())
        .collect()
}

pub fn assert_fail(err: Error, needle: &str) {
    assert_eq!(err.exit_code(), 1, "{err}");
    assert!(
        err.to_string().contains(needle),
        "expected {needle:?} in {err}"
    );
}

pub fn assert_usage(err: Error, needle: &str) {
    assert_eq!(err.exit_code(), 2, "{err}");
    assert!(
        err.to_string().contains(needle),
        "expected {needle:?} in {err}"
    );
}
