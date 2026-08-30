use jiff::tz::TimeZone;

use crate::error::Error;
use crate::ledger::{
    Agent, Entries, Entry, FieldValue, GroupedLink, GroupedTime, Outcome, Posted, Schemas, Stamp,
    Total,
};
use crate::spec::{EnumValue, FieldKind, Link, Spec};
use crate::time;

use super::Style;
use super::tsv;

pub fn render(outcome: &Outcome, tz: &TimeZone, style: Style) -> Result<String, Error> {
    let header = style != Style::TsvNoHeader;
    match outcome {
        Outcome::Empty => Ok(String::new()),
        Outcome::Schemas(Schemas { schemas }) => {
            let rows: Vec<Vec<String>> = schemas
                .iter()
                .map(|s| vec![s.name.to_string(), tsv::bool_cell(s.retired).to_string()])
                .collect();
            Ok(tsv::table(&["name", "retired"], &rows, header))
        }
        Outcome::Spec(spec) => match style {
            Style::Yaml => spec.to_yaml(),
            Style::Tsv | Style::TsvNoHeader => render_spec(spec, header),
        },
        Outcome::Entries(Entries {
            spec,
            entries,
            include_ignored,
        }) => render_entries(spec, entries, *include_ignored, tz, header),
        Outcome::Posted(rows) => {
            let mut out = Vec::new();
            for Posted { id, at, links } in rows {
                out.push(vec![
                    id.to_string(),
                    time::display_at(*at, tz)?,
                    render_links(links),
                ]);
            }
            Ok(tsv::table(&["id", "at", "links"], &out, header))
        }
        Outcome::Stamp(Stamp { id, at }) => {
            let at = time::display_at(*at, tz)?;
            Ok(tsv::table(
                &["id", "at"],
                &[vec![id.to_string(), at]],
                header,
            ))
        }
        Outcome::Total(Total { field, value }) => Ok(tsv::table(
            &["field", "value"],
            &[vec![field.to_string(), tsv::number(*value)]],
            header,
        )),
        Outcome::GroupedTime(GroupedTime { unit, buckets }) => {
            let rows: Vec<Vec<String>> = buckets
                .iter()
                .map(|(k, v)| vec![k.to_string(), tsv::number(*v)])
                .collect();
            Ok(tsv::table(&[unit.as_str(), "value"], &rows, header))
        }
        Outcome::GroupedLink(GroupedLink { name, buckets }) => {
            let rows: Vec<Vec<String>> = buckets
                .iter()
                .map(|(k, v)| {
                    vec![
                        k.as_ref().map(ToString::to_string).unwrap_or_default(),
                        tsv::number(*v),
                    ]
                })
                .collect();
            Ok(tsv::table(&[name.as_str(), "value"], &rows, header))
        }
    }
}

fn render_spec(spec: &Spec, header: bool) -> Result<String, Error> {
    let mut rows = Vec::new();
    for field in &spec.fields {
        let values = match &field.kind {
            FieldKind::Enum(v) => v
                .iter()
                .map(EnumValue::as_str)
                .collect::<Vec<_>>()
                .join(","),
            _ => String::new(),
        };
        rows.push(vec![
            field.name.to_string(),
            type_name(&field.kind).to_string(),
            tsv::bool_cell(field.required).to_string(),
            values,
        ]);
    }
    Ok(tsv::table(
        &["name", "type", "required", "values"],
        &rows,
        header,
    ))
}

fn render_entries(
    spec: &Spec,
    entries: &[Entry],
    show_ignored: bool,
    tz: &TimeZone,
    header: bool,
) -> Result<String, Error> {
    let mut headers = vec!["id".to_string(), "at".to_string(), "links".to_string()];
    for field in &spec.fields {
        headers.push(field.name.to_string());
    }
    headers.push("agent".to_string());
    if show_ignored {
        headers.push("ignored".to_string());
    }
    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    let mut out_rows = Vec::new();
    for entry in entries {
        let mut cells = vec![
            entry.id.to_string(),
            time::display_at(entry.at, tz)?,
            render_links(&entry.links),
        ];
        for field in &spec.fields {
            cells.push(render_value(entry.values.get(&field.name)));
        }
        cells.push(
            entry
                .agent
                .as_ref()
                .map(Agent::as_str)
                .unwrap_or_default()
                .to_string(),
        );
        if show_ignored {
            cells.push(tsv::bool_cell(entry.ignored).to_string());
        }
        out_rows.push(cells);
    }
    Ok(tsv::table(&header_refs, &out_rows, header))
}

fn render_value(value: Option<&FieldValue>) -> String {
    match value {
        None | Some(FieldValue::Empty) => String::new(),
        Some(FieldValue::Text(s)) => s.clone(),
        Some(FieldValue::Number(n)) => tsv::number(*n),
        Some(FieldValue::Enum(v)) => v.to_string(),
    }
}

fn render_links(links: &[Link]) -> String {
    links
        .iter()
        .map(|l| format!("{}={}", l.name, l.to))
        .collect::<Vec<_>>()
        .join(" ")
}

fn type_name(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Text => "text",
        FieldKind::Number => "number",
        FieldKind::Enum(_) => "enum",
    }
}
