pub fn bool_cell(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

pub fn number(n: rust_decimal::Decimal) -> String {
    n.to_string()
}

pub fn table(headers: &[&str], rows: &[Vec<String>], header: bool) -> String {
    let mut out = String::new();
    if header {
        out.push_str(&headers.join("\t"));
        out.push('\n');
    }
    for row in rows {
        out.push_str(&row.join("\t"));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_can_omit_header() {
        let rows = [vec!["1".into(), "a".into()]];
        assert_eq!(table(&["id", "x"], &rows, true), "id\tx\n1\ta\n");
        assert_eq!(table(&["id", "x"], &rows, false), "1\ta\n");
        assert_eq!(table(&["id", "x"], &[], false), "");
        assert_eq!(table(&["id", "x"], &[], true), "id\tx\n");
    }
}
