use std::fmt;

use crate::error::{Error, Fail, Usage};

pub fn cli_message(err: &Error) -> String {
    match err {
        Error::Usage(err) => usage_message(err),
        Error::Fail(err) => fail_message(err),
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
        Usage::EmptyFilter(name) => format!("empty {name} filter"),
        Usage::DuplicateLinkName(name) => format!("duplicate link name: {name}"),
        Usage::DuplicateField(name) => format!("duplicate field: {name}"),
        Usage::InvalidLinkTarget(s) => format!("invalid link target: {s}"),
        Usage::DateOnlyNotInstant => "date-only is a query bound, not an instant".into(),
        Usage::TimeMustUseT => "time must use T, not a space".into(),
        Usage::InvalidDate(input) => format!("invalid date: {input}"),
        Usage::InvalidTime(input) => format!("invalid time: {input}"),
        Usage::OffsetNeedsColon => "offset must include a colon (+10:00)".into(),
        Usage::UnknownHelpTopic(topic) => format!("unknown help topic: {topic}"),
        Usage::UnknownType(t) => format!("unknown type: {t}"),
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
        Fail::ValuesOnlyForEnum(name) => format!("values only apply to enum, not {name}"),
        Fail::EnumNeedsValues(name) => format!("enum {name} needs values"),
        Fail::CorruptSchemaName(name) => format!("corrupt schema name: {name}"),
        Fail::CorruptLinkName(name) => format!("corrupt link name: {name}"),
        Fail::CorruptLinkSchema(name) => format!("corrupt link schema: {name}"),
        Fail::CorruptStoredTime(raw) => format!("corrupt stored time: {raw}"),
        Fail::CorruptStoredNumber(raw) => format!("corrupt stored number: {raw}"),
        Fail::CorruptStoredEnum(raw) => format!("corrupt stored enum: {raw}"),
        Fail::CorruptStoredAgent(raw) => format!("corrupt stored agent: {raw}"),
        Fail::HomeNotSet => "HOME is not set".into(),
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&cli_message(self))
    }
}
