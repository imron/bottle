use std::path::PathBuf;

use crate::spec::FieldType;

#[derive(Debug, Clone)]
pub enum Cmd {
    Help {
        topic: Option<String>,
    },
    SchemaList,
    SchemaShow {
        name: String,
        yaml: bool,
    },
    SchemaAdd {
        name: String,
        file: PathBuf,
    },
    SchemaAddField {
        schema: String,
        name: String,
        type_: FieldType,
        values: Option<Vec<String>>,
        default: Option<String>,
    },
    SchemaAddValue {
        schema: String,
        field: String,
        value: String,
    },
    SchemaRetire {
        name: String,
    },
    SchemaDrop {
        name: String,
    },
    Log {
        schema: String,
        at: Option<String>,
        agent: Option<String>,
        links: Vec<(String, String)>,
        fields: Vec<(String, String)>,
    },
    Ls {
        schema: String,
        from: Option<String>,
        to: Option<String>,
        agent: Option<String>,
        wheres: Vec<(String, String)>,
        include_ignored: bool,
    },
    Get {
        schema: String,
        id: i64,
    },
    Sum {
        schema: String,
        field: String,
        from: Option<String>,
        to: Option<String>,
        agent: Option<String>,
        wheres: Vec<(String, String)>,
        group: Option<String>,
    },
    Last {
        schema: String,
        agent: Option<String>,
        wheres: Vec<(String, String)>,
    },
    Today {
        schema: String,
        agent: Option<String>,
        wheres: Vec<(String, String)>,
    },
    Amend {
        schema: String,
        id: i64,
        at: Option<String>,
        agent: Option<String>,
        links: Vec<(String, String)>,
        unlinks: Vec<String>,
        fields: Vec<(String, String)>,
    },
    Ignore {
        schema: String,
        id: i64,
    },
}
