use std::path::PathBuf;

use bottle::{Bottle, Cmd, Style, cmd, execute, parse};

use crate::common::{TZ, assert_fail, assert_usage, harness, seed_meals, tsv_lines};

#[test]
fn backup_copies_entries() {
    let mut h = harness();
    seed_meals(&mut h);
    let dest = h.dir.path().join("copy.db");
    let out = h.run_ok(Cmd::Backup(cmd::Backup { path: dest.clone() }));
    assert!(out.is_empty(), "{out}");
    assert!(dest.is_file());
    assert!(!dest.with_extension("db-wal").exists());
    let mut copy = Bottle::open(&dest, Some("test".into()), Some(TZ)).unwrap();
    let tz = copy.tz().clone();
    let request = parse(
        Cmd::Ls(cmd::Ls {
            filters: cmd::Filters {
                schema: "nutrition.meal".into(),
                agent: None,
                wheres: vec![],
                links: vec![],
                excludes: vec![],
            },
            from: None,
            to: None,
            include_ignored: false,
        }),
        &tz,
    )
    .unwrap();
    let ls = execute(&mut copy, request, Style::Tsv).unwrap();
    assert_eq!(tsv_lines(&ls).len(), 3);
}

#[test]
fn backup_rejects_existing_path() {
    let mut h = harness();
    let dest = h.dir.path().join("copy.db");
    h.run_ok(Cmd::Backup(cmd::Backup { path: dest.clone() }));
    let err = h
        .run(Cmd::Backup(cmd::Backup { path: dest.clone() }))
        .unwrap_err();
    assert_fail(err, "file exists");
}

#[test]
fn backup_rejects_missing_parent() {
    let mut h = harness();
    let dest = h.dir.path().join("nope").join("copy.db");
    let err = h.run(Cmd::Backup(cmd::Backup { path: dest })).unwrap_err();
    assert_fail(err, "file not found");
}

#[test]
fn backup_rejects_empty_path() {
    let mut h = harness();
    let err = h
        .run(Cmd::Backup(cmd::Backup {
            path: PathBuf::new(),
        }))
        .unwrap_err();
    assert_usage(err, "backup requires a path");
}
