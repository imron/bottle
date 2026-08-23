use crate::error::Error;

pub fn page(topic: Option<&str>) -> Result<String, Error> {
    let topic = topic.unwrap_or("overview");
    let body = match topic {
        "overview" => include_str!("../docs/help/overview.md"),
        "help" => include_str!("../docs/help/help.md"),
        "schema" => include_str!("../docs/help/schema.md"),
        "schema list" => include_str!("../docs/help/schema-list.md"),
        "schema show" => include_str!("../docs/help/schema-show.md"),
        "schema add" => include_str!("../docs/help/schema-add.md"),
        "schema add-field" => include_str!("../docs/help/schema-add-field.md"),
        "schema add-value" => include_str!("../docs/help/schema-add-value.md"),
        "schema retire" => include_str!("../docs/help/schema-retire.md"),
        "schema drop" => include_str!("../docs/help/schema-drop.md"),
        "log" => include_str!("../docs/help/log.md"),
        "ls" => include_str!("../docs/help/ls.md"),
        "get" => include_str!("../docs/help/get.md"),
        "sum" => include_str!("../docs/help/sum.md"),
        "last" => include_str!("../docs/help/last.md"),
        "today" => include_str!("../docs/help/today.md"),
        "amend" => include_str!("../docs/help/amend.md"),
        "ignore" => include_str!("../docs/help/ignore.md"),
        "mcp" => include_str!("../docs/help/mcp.md"),
        _ => return Err(Error::usage(format!("unknown help topic: {topic}"))),
    };
    let mut out = body.trim().to_string();
    out.push('\n');
    Ok(out)
}
