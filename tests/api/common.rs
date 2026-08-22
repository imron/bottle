use std::path::PathBuf;
use std::sync::Once;

use bottle::{Bottle, Cmd, Error};
use tempfile::TempDir;

static PIN_TZ: Once = Once::new();

pub struct Harness {
    pub dir: TempDir,
    bottle: Bottle,
}

pub fn harness() -> Harness {
    PIN_TZ.call_once(|| {
        // Safety: set once before any jiff system-zone read in these tests.
        unsafe {
            std::env::set_var("TZ", "Australia/Sydney");
        }
    });
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("bottle.db");
    let bottle = Bottle::open(&db, Some("test".into())).unwrap();
    Harness { dir, bottle }
}

impl Harness {
    pub fn run(&mut self, cmd: Cmd) -> Result<String, Error> {
        self.bottle.run(cmd)
    }

    pub fn run_ok(&mut self, cmd: Cmd) -> String {
        self.run(cmd).unwrap_or_else(|e| panic!("{e}"))
    }

    pub fn yaml_file(&self, name: &str, body: &str) -> PathBuf {
        let path = self.dir.path().join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    pub fn add_schema(&mut self, name: &str, yaml: &str) {
        let file = self.yaml_file(&format!("{name}.yaml"), yaml);
        self.run_ok(Cmd::SchemaAdd {
            name: name.into(),
            file,
        });
    }

    pub fn log(
        &mut self,
        schema: &str,
        fields: &[(&str, &str)],
        links: &[(&str, &str)],
        at: Option<&str>,
    ) -> String {
        self.run_ok(Cmd::Log {
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
        })
    }
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
        err.message().contains(needle),
        "expected {needle:?} in {}",
        err.message()
    );
}

pub fn assert_usage(err: Error, needle: &str) {
    assert_eq!(err.exit_code(), 2, "{err}");
    assert!(
        err.message().contains(needle),
        "expected {needle:?} in {}",
        err.message()
    );
}
