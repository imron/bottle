use crate::error::Error;

const HELP: &str = include_str!("../docs/help.md");

pub fn page(topic: Option<&str>) -> Result<String, Error> {
    let topic = topic.unwrap_or("overview");
    let want = format!("## {topic}");
    for section in HELP.split("\n---\n") {
        let section = section.trim();
        let heading = section.lines().next().unwrap_or("");
        if heading == want {
            let mut out = section.to_string();
            if !out.ends_with('\n') {
                out.push('\n');
            }
            return Ok(out);
        }
    }
    Err(Error::usage(format!("unknown help topic: {topic}")))
}
