pub fn bool_cell(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

pub fn number(n: f64) -> String {
    if n == 0.0 {
        return "0".to_string();
    }
    let s = format!("{n:.10}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
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
