use crate::error::{Error, Usage};

pub fn page(topic: Option<&str>) -> Result<String, Error> {
    let topic = topic.unwrap_or("overview");
    let body = match topic {
        "overview" => include_str!("overview.md"),
        "help" => include_str!("help.md"),
        "schema" => include_str!("schema.md"),
        "schema list" => include_str!("schema-list.md"),
        "schema show" => include_str!("schema-show.md"),
        "schema add" => include_str!("schema-add.md"),
        "schema add-field" => include_str!("schema-add-field.md"),
        "schema add-value" => include_str!("schema-add-value.md"),
        "schema retire" => include_str!("schema-retire.md"),
        "schema drop" => include_str!("schema-drop.md"),
        "log" => include_str!("log.md"),
        "ls" => include_str!("ls.md"),
        "get" => include_str!("get.md"),
        "sum" => include_str!("sum.md"),
        "last" => include_str!("last.md"),
        "today" => include_str!("today.md"),
        "amend" => include_str!("amend.md"),
        "ignore" => include_str!("ignore.md"),
        "unignore" => include_str!("unignore.md"),
        "backup" => include_str!("backup.md"),
        "mcp" => include_str!("mcp.md"),
        _ => return Err(Error::Usage(Usage::UnknownHelpTopic(topic.to_string()))),
    };
    Ok(body.to_string())
}
