use std::fmt;

use crate::spec::{EntryId, EntryRef, EnumValue, FieldName, Identifier, LinkName, SchemaName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Usage(Usage),
    Fail(Fail),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Usage {
    EnumValuesRequired,
    EnumValuesNotAllowed,
    EmptyLog,
    AmendEmpty,
    DuplicateUnlink(LinkName),
    LinkAndUnlink(LinkName),
    ReservedWhere(Identifier),
    EmptyValue(FieldName),
    DuplicateLinkName(LinkName),
    DuplicateField(FieldName),
    InvalidLinkTarget(String),
    InvalidEntryId(i64),
    DateOnlyNotInstant,
    TimeMustUseT,
    InvalidDate(String),
    InvalidTime(String),
    OffsetNeedsColon,
    UnknownHelpTopic(String),
    UnknownType(String),
    EmptyBackupPath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fail {
    SchemaRetired(SchemaName),
    FieldExists(FieldName),
    UnknownField(FieldName),
    FieldNotEnum(FieldName),
    EnumValueExists(EnumValue),
    UnknownSchema(SchemaName),
    SchemaHasInboundLinks(SchemaName),
    SchemaExists(SchemaName),
    EntryNotFound { schema: SchemaName, id: EntryId },
    NotFound,
    FieldNotNumber(FieldName),
    LinkNameCollidesWithField(LinkName),
    LinkTargetMissing(EntryRef),
    MissingRequiredField(FieldName),
    TextHasTabOrNewline(FieldName),
    EnumHasTabNewlineOrComma,
    AgentHasTabOrNewline,
    EmptyAgent,
    InvalidEnumValue { field: FieldName, value: String },
    InvalidSpec(String),
    Yaml(String),
    InvalidSchemaName(String),
    InvalidFieldName(String),
    ReservedFieldName(String),
    InvalidLinkName(String),
    ReservedLinkName(String),
    EmptyEnumValue,
    DuplicateEnumValue(EnumValue),
    DuplicateSpecField(FieldName),
    InvalidNumber(String),
    NumberOverflow,
    ValuesOnlyForEnum(FieldName),
    EnumNeedsValues(FieldName),
    InvalidIdentifier(String),
    CorruptSchemaName(String),
    CorruptLinkName(String),
    CorruptLinkSchema(String),
    CorruptStoredTime(String),
    CorruptStoredNumber(String),
    CorruptStoredEnum(String),
    CorruptStoredAgent(String),
    CorruptStoredText(String),
    CorruptStoredId(i64),
    UnsupportedStoreVersion(i32),
    HomeNotSet,
    FileNotFound(String),
    FileExists(String),
    Store(String),
    Io(String),
    Time(String),
}

impl Error {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Usage(_) => 2,
            Self::Fail(_) => 1,
        }
    }
}

fn usage_message(err: &Usage) -> String {
    match err {
        Usage::EnumValuesRequired => "values is required for enum".into(),
        Usage::EnumValuesNotAllowed => "values is only valid for enum".into(),
        Usage::EmptyLog => "log requires at least one entry".into(),
        Usage::AmendEmpty => "amend requires at least one change".into(),
        Usage::DuplicateUnlink(name) => format!("duplicate unlink {name}"),
        Usage::LinkAndUnlink(name) => format!("cannot link and unlink {name}"),
        Usage::ReservedWhere(name) => {
            format!("{name} is reserved; use agent, get, or from/to")
        }
        Usage::EmptyValue(name) => format!("empty {name}"),
        Usage::DuplicateLinkName(name) => format!("duplicate link name: {name}"),
        Usage::DuplicateField(name) => format!("duplicate field: {name}"),
        Usage::InvalidLinkTarget(s) => format!("invalid link target: {s}"),
        Usage::InvalidEntryId(id) => format!("invalid id: {id}"),
        Usage::DateOnlyNotInstant => "date-only is a query bound, not an instant".into(),
        Usage::TimeMustUseT => "time must use T, not a space".into(),
        Usage::InvalidDate(input) => format!("invalid date: {input}"),
        Usage::InvalidTime(input) => format!("invalid time: {input}"),
        Usage::OffsetNeedsColon => "offset must include a colon (+10:00)".into(),
        Usage::UnknownHelpTopic(topic) => format!("unknown help topic: {topic}"),
        Usage::UnknownType(t) => format!("unknown type: {t}"),
        Usage::EmptyBackupPath => "backup requires a path".into(),
    }
}

fn fail_message(err: &Fail) -> String {
    match err {
        Fail::SchemaRetired(name) => format!("schema is retired: {name}"),
        Fail::FieldExists(name) => format!("field exists: {name}"),
        Fail::UnknownField(name) => format!("unknown field: {name}"),
        Fail::FieldNotEnum(name) => format!("field is not enum: {name}"),
        Fail::EnumValueExists(value) => format!("enum value exists: {value}"),
        Fail::UnknownSchema(name) => format!("unknown schema: {name}"),
        Fail::SchemaHasInboundLinks(name) => format!("schema {name} still has inbound links"),
        Fail::SchemaExists(name) => format!("schema exists: {name}"),
        Fail::EntryNotFound { schema, id } => format!("not found: {schema}/{id}"),
        Fail::NotFound => "not found".into(),
        Fail::FieldNotNumber(name) => format!("field is not a number: {name}"),
        Fail::LinkNameCollidesWithField(name) => format!("link name collides with field: {name}"),
        Fail::LinkTargetMissing(to) => format!("link target missing: {to}"),
        Fail::MissingRequiredField(name) => format!("missing required field: {name}"),
        Fail::TextHasTabOrNewline(name) => format!("text {name} may not contain tab or newline"),
        Fail::EnumHasTabNewlineOrComma => {
            "enum value may not contain tab, newline, or comma".into()
        }
        Fail::AgentHasTabOrNewline => "agent may not contain tab or newline".into(),
        Fail::EmptyAgent => "empty agent".into(),
        Fail::InvalidEnumValue { field, value } => format!("invalid {field} value: {value}"),
        Fail::InvalidSpec(e) => format!("invalid spec: {e}"),
        Fail::Yaml(e) => format!("yaml error: {e}"),
        Fail::Store(e) => format!("store error: {e}"),
        Fail::Io(e) => format!("io error: {e}"),
        Fail::Time(e) => format!("time error: {e}"),
        Fail::FileNotFound(path) => format!("file not found: {path}"),
        Fail::FileExists(path) => format!("file exists: {path}"),
        Fail::InvalidSchemaName(s) => format!("invalid schema name: {s}"),
        Fail::InvalidFieldName(s) => format!("invalid field name: {s}"),
        Fail::ReservedFieldName(s) => format!("reserved field name: {s}"),
        Fail::InvalidLinkName(s) => format!("invalid link name: {s}"),
        Fail::ReservedLinkName(s) => format!("reserved link name: {s}"),
        Fail::InvalidIdentifier(s) => format!("invalid name: {s}"),
        Fail::EmptyEnumValue => "empty enum value".into(),
        Fail::DuplicateEnumValue(value) => format!("duplicate enum value after fold: {value}"),
        Fail::DuplicateSpecField(name) => format!("duplicate field: {name}"),
        Fail::InvalidNumber(raw) => format!("invalid number: {raw}"),
        Fail::NumberOverflow => "number overflow".into(),
        Fail::ValuesOnlyForEnum(name) => format!("values only apply to enum, not {name}"),
        Fail::EnumNeedsValues(name) => format!("enum {name} needs values"),
        Fail::CorruptSchemaName(name) => format!("corrupt schema name: {name}"),
        Fail::CorruptLinkName(name) => format!("corrupt link name: {name}"),
        Fail::CorruptLinkSchema(name) => format!("corrupt link schema: {name}"),
        Fail::CorruptStoredTime(raw) => format!("corrupt stored time: {raw}"),
        Fail::CorruptStoredNumber(raw) => format!("corrupt stored number: {raw}"),
        Fail::CorruptStoredEnum(raw) => format!("corrupt stored enum: {raw}"),
        Fail::CorruptStoredAgent(raw) => format!("corrupt stored agent: {raw}"),
        Fail::CorruptStoredText(raw) => format!("corrupt stored text: {raw}"),
        Fail::CorruptStoredId(id) => format!("corrupt stored id: {id}"),
        Fail::UnsupportedStoreVersion(v) => format!("unsupported store version: {v}"),
        Fail::HomeNotSet => "HOME is not set".into(),
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(err) => f.write_str(&usage_message(err)),
            Self::Fail(err) => f.write_str(&fail_message(err)),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Fail(Fail::Io(err.to_string()))
    }
}

impl From<jiff::Error> for Error {
    fn from(err: jiff::Error) -> Self {
        Self::Fail(Fail::Time(err.to_string()))
    }
}
