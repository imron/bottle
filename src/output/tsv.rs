pub fn bool_cell(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

pub fn number(n: rust_decimal::Decimal) -> String {
    n.to_string()
}

pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = headers.join("\t");
    out.push('\n');
    for row in rows {
        out.push_str(&row.join("\t"));
        out.push('\n');
    }
    out
}
