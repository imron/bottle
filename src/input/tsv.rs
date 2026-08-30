use std::collections::HashSet;

use super::LogInput;
use crate::error::{Error, Usage};

pub fn log_rows(schema: &str, raw: &str) -> Result<Vec<LogInput>, Error> {
    let mut lines: Vec<&str> = raw
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    let Some(header) = lines.first() else {
        return Err(Error::Usage(Usage::LogFileNeedsHeader));
    };
    if header.is_empty() {
        return Err(Error::Usage(Usage::LogFileNeedsHeader));
    }
    let cols: Vec<&str> = header.split('\t').collect();
    if cols.iter().any(|c| c.is_empty()) {
        return Err(Error::Usage(Usage::LogFileNeedsHeader));
    }
    let mut seen = HashSet::new();
    for name in &cols {
        if !seen.insert(*name) {
            return Err(Error::Usage(Usage::DuplicateHeader((*name).to_string())));
        }
    }
    let rows = &lines[1..];
    if rows.is_empty() {
        return Err(Error::Usage(Usage::EmptyLog));
    }
    let mut out = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let line = i + 2;
        let cells = cells_for_header(row, cols.len(), line)?;
        out.push(row_entry(schema, &cols, &cells, line).map_err(|e| e.at_file_line(Some(line)))?);
    }
    Ok(out)
}

fn cells_for_header(row: &str, header: usize, line: usize) -> Result<Vec<&str>, Error> {
    let cells: Vec<&str> = row.split('\t').collect();
    if cells.len() < header
        || (cells.len() > header && cells[header..].iter().any(|c| !c.is_empty()))
    {
        return Err(Error::Usage(Usage::TsvRowWidth {
            columns: cells.len(),
            header,
        })
        .at_file_line(Some(line)));
    }
    if cells.len() > header {
        return Ok(cells[..header].to_vec());
    }
    Ok(cells)
}

fn row_entry(schema: &str, cols: &[&str], cells: &[&str], line: usize) -> Result<LogInput, Error> {
    let mut at = None;
    let mut agent = None;
    let mut links = Vec::new();
    let mut fields = Vec::new();
    for (name, value) in cols.iter().zip(cells.iter()) {
        match *name {
            "at" => {
                if !value.is_empty() {
                    at = Some((*value).to_string());
                }
            }
            "agent" => {
                if !value.is_empty() {
                    agent = Some((*value).to_string());
                }
            }
            "links" => links = links_cell(value)?,
            other => fields.push((other.to_string(), (*value).to_string())),
        }
    }
    Ok(LogInput {
        schema: schema.to_string(),
        at,
        agent,
        links,
        fields,
        file_line: Some(line),
    })
}

fn links_cell(cell: &str) -> Result<Vec<(String, String)>, Error> {
    let mut out = Vec::new();
    for part in cell.split(' ') {
        if part.is_empty() {
            continue;
        }
        match part.split_once('=') {
            Some((name, target)) if !name.is_empty() => {
                out.push((name.to_string(), target.to_string()));
            }
            _ => return Err(Error::Usage(Usage::InvalidLinkTarget(part.to_string()))),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_optional_at_agent_and_links() {
        let rows = log_rows(
            "fitness.set",
            "at\tagent\tlinks\tmovement\treps\n\
             \t\tsession=fitness.session/1\tsquat\t8\n",
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].at.is_none());
        assert!(rows[0].agent.is_none());
        assert_eq!(
            rows[0].links,
            vec![("session".into(), "fitness.session/1".into())]
        );
        assert_eq!(
            rows[0].fields,
            vec![
                ("movement".into(), "squat".into()),
                ("reps".into(), "8".into())
            ]
        );
    }

    #[test]
    fn trailing_newline_is_not_a_row() {
        let rows = log_rows("meal", "what\neggs\n").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].fields, vec![("what".into(), "eggs".into())]);
    }

    #[test]
    fn empty_header_line_is_usage() {
        let err = log_rows("meal", "\nwhat\neggs\n").unwrap_err();
        assert!(matches!(err, Error::Usage(Usage::LogFileNeedsHeader)));
    }

    #[test]
    fn empty_header_cell_is_usage() {
        let err = log_rows("meal", "when\t\tkcal\nbreakfast\tx\t1\n").unwrap_err();
        assert!(matches!(err, Error::Usage(Usage::LogFileNeedsHeader)));
    }

    #[test]
    fn short_row_is_wrong_width() {
        let err = log_rows("meal", "when\twhat\nbreakfast\n").unwrap_err();
        assert_eq!(
            err.to_string(),
            "log --file line 2: 1 columns, header has 2"
        );
    }

    #[test]
    fn trailing_empty_cells_are_not_extra_columns() {
        let rows = log_rows("meal", "when\twhat\nbreakfast\teggs\t\t\n").unwrap();
        assert_eq!(
            rows[0].fields,
            vec![
                ("when".into(), "breakfast".into()),
                ("what".into(), "eggs".into())
            ]
        );
        let err = log_rows("meal", "when\twhat\nbreakfast\teggs\t\tnope\n").unwrap_err();
        assert_eq!(
            err.to_string(),
            "log --file line 2: 4 columns, header has 2"
        );
    }

    #[test]
    fn agent_cell_is_kept() {
        let rows = log_rows("meal", "agent\twhat\ncoach\teggs\n").unwrap();
        assert_eq!(rows[0].agent.as_deref(), Some("coach"));
    }

    #[test]
    fn links_skip_double_space_and_reject_bare_name() {
        let rows = log_rows(
            "set",
            "links\n\
             session=fitness.session/1  project=work.project/2\n",
        )
        .unwrap();
        assert_eq!(
            rows[0].links,
            vec![
                ("session".into(), "fitness.session/1".into()),
                ("project".into(), "work.project/2".into())
            ]
        );
        let err = log_rows("set", "links\nsession\n").unwrap_err();
        assert_eq!(
            err.to_string(),
            "log --file line 2: invalid link target: session"
        );
    }

    #[test]
    fn crlf_rows_parse() {
        let rows = log_rows("meal", "what\r\neggs\r\n").unwrap();
        assert_eq!(rows[0].fields, vec![("what".into(), "eggs".into())]);
    }
}
