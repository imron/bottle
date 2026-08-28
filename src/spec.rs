use std::collections::HashSet;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Fail, Usage};

macro_rules! string_newtype {
    ($name:ident) => {
        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
    ($name:ident, from_str) => {
        string_newtype!($name);

        impl std::str::FromStr for $name {
            type Err = Error;
            fn from_str(s: &str) -> Result<Self, Error> {
                Self::parse(s)
            }
        }

        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                deserialize_from_str(deserializer)
            }
        }
    };
}

pub(crate) use string_newtype;

fn deserialize_from_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone)]
pub struct Spec {
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: FieldName,
    pub kind: FieldKind,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Number,
    Enum(Vec<EnumValue>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FromTypeErr {
    ValuesRequired,
    ValuesNotAllowed,
    Duplicate(EnumValue),
}

impl FieldKind {
    pub fn from_type(
        type_: FieldType,
        values: Option<Vec<EnumValue>>,
    ) -> Result<Self, FromTypeErr> {
        match (type_, values) {
            (FieldType::Text, None) => Ok(Self::Text),
            (FieldType::Number, None) => Ok(Self::Number),
            (FieldType::Enum, Some(values)) if !values.is_empty() => {
                let mut seen = HashSet::new();
                for value in &values {
                    if !seen.insert(value.clone()) {
                        return Err(FromTypeErr::Duplicate(value.clone()));
                    }
                }
                Ok(Self::Enum(values))
            }
            (FieldType::Enum, _) => Err(FromTypeErr::ValuesRequired),
            (FieldType::Text | FieldType::Number, Some(_)) => Err(FromTypeErr::ValuesNotAllowed),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Number,
    Enum,
}

impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Enum => "enum",
        })
    }
}

impl std::str::FromStr for FieldType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        match s {
            "text" => Ok(Self::Text),
            "number" => Ok(Self::Number),
            "enum" => Ok(Self::Enum),
            other => Err(Error::Usage(Usage::UnknownType(other.to_string()))),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpecDoc {
    fields: Vec<FieldDoc>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldDoc {
    name: FieldName,
    #[serde(rename = "type")]
    type_: FieldType,
    required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    values: Option<Vec<EnumValue>>,
}

impl Spec {
    pub fn parse_yaml(raw: &str) -> Result<Self, Error> {
        let doc: SpecDoc =
            serde_yaml::from_str(raw).map_err(|e| Error::Fail(Fail::InvalidSpec(e.to_string())))?;
        Self::from_doc(doc)
    }

    pub fn to_yaml(&self) -> Result<String, Error> {
        let doc = SpecDoc {
            fields: self.fields.iter().map(Field::to_doc).collect(),
        };
        match serde_yaml::to_string(&doc) {
            Ok(yaml) => Ok(yaml),
            Err(err) => Err(Error::Fail(Fail::Yaml(err.to_string()))),
        }
    }

    pub fn field(&self, name: &FieldName) -> Option<&Field> {
        self.fields.iter().find(|f| &f.name == name)
    }

    pub fn ensure_link_name(&self, name: &LinkName) -> Result<(), Error> {
        if self.fields.iter().any(|f| f.name.as_str() == name.as_str()) {
            Err(Error::Fail(Fail::LinkNameCollidesWithField(name.clone())))
        } else {
            Ok(())
        }
    }

    fn from_doc(doc: SpecDoc) -> Result<Self, Error> {
        let mut seen = HashSet::new();
        let mut fields = Vec::new();
        for field in doc.fields {
            let field = Field::from_doc(field)?;
            if !seen.insert(field.name.clone()) {
                return Err(Error::Fail(Fail::DuplicateSpecField(field.name)));
            }
            fields.push(field);
        }
        Ok(Self { fields })
    }
}

impl Field {
    fn from_doc(doc: FieldDoc) -> Result<Self, Error> {
        let kind = FieldKind::from_type(doc.type_, doc.values).map_err(|e| match e {
            FromTypeErr::ValuesRequired => Error::Fail(Fail::EnumNeedsValues(doc.name.clone())),
            FromTypeErr::ValuesNotAllowed => Error::Fail(Fail::ValuesOnlyForEnum(doc.name.clone())),
            FromTypeErr::Duplicate(v) => Error::Fail(Fail::DuplicateEnumValue(v)),
        })?;
        Ok(Self {
            name: doc.name,
            kind,
            required: doc.required,
        })
    }

    fn to_doc(&self) -> FieldDoc {
        let (type_, values) = match &self.kind {
            FieldKind::Text => (FieldType::Text, None),
            FieldKind::Number => (FieldType::Number, None),
            FieldKind::Enum(values) => (FieldType::Enum, Some(values.clone())),
        };
        FieldDoc {
            name: self.name.clone(),
            type_,
            required: self.required,
            values,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier(String);

impl Identifier {
    pub fn parse(s: &str) -> Result<Self, Error> {
        if is_identifier(s) {
            Ok(Self(s.to_string()))
        } else {
            Err(Error::Fail(Fail::InvalidIdentifier(s.to_string())))
        }
    }
}

string_newtype!(Identifier, from_str);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaName(String);

impl SchemaName {
    pub fn parse(s: &str) -> Result<Self, Error> {
        let folded = s.replace('_', ".");
        if is_schema_name(&folded) {
            Ok(Self(folded))
        } else {
            Err(Error::Fail(Fail::InvalidSchemaName(s.to_string())))
        }
    }
}

string_newtype!(SchemaName, from_str);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldName(String);

impl FieldName {
    pub fn parse(s: &str) -> Result<Self, Error> {
        if !is_identifier(s) {
            return Err(Error::Fail(Fail::InvalidFieldName(s.to_string())));
        }
        if is_reserved(s) {
            return Err(Error::Fail(Fail::ReservedFieldName(s.to_string())));
        }
        Ok(Self(s.to_string()))
    }
}

string_newtype!(FieldName, from_str);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LinkName(String);

impl LinkName {
    pub fn parse(s: &str) -> Result<Self, Error> {
        if !is_identifier(s) {
            return Err(Error::Fail(Fail::InvalidLinkName(s.to_string())));
        }
        if is_reserved(s) || is_time_group(s) {
            return Err(Error::Fail(Fail::ReservedLinkName(s.to_string())));
        }
        Ok(Self(s.to_string()))
    }
}

string_newtype!(LinkName, from_str);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EnumValue(String);

impl EnumValue {
    pub fn parse(s: &str) -> Result<Self, Error> {
        let folded = s.to_ascii_lowercase();
        if folded.is_empty() {
            return Err(Error::Fail(Fail::EmptyEnumValue));
        }
        if folded.contains('\t') || folded.contains('\n') || folded.contains(',') {
            return Err(Error::Fail(Fail::EnumHasTabNewlineOrComma));
        }
        Ok(Self(folded))
    }
}

string_newtype!(EnumValue, from_str);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimePeriod {
    Day,
    Week,
    Month,
    Year,
}

impl TimePeriod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::Year => "year",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Group {
    Time(TimePeriod),
    Link(LinkName),
}

impl Group {
    pub fn parse(s: &str) -> Result<Self, Error> {
        match s {
            "day" => Ok(Self::Time(TimePeriod::Day)),
            "week" => Ok(Self::Time(TimePeriod::Week)),
            "month" => Ok(Self::Time(TimePeriod::Month)),
            "year" => Ok(Self::Time(TimePeriod::Year)),
            other => Ok(Self::Link(LinkName::parse(other)?)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryId(i64);

impl EntryId {
    pub fn parse(n: i64) -> Result<Self, Error> {
        Self::from_raw(n).ok_or(Error::Usage(Usage::InvalidEntryId(n)))
    }

    pub fn as_i64(self) -> i64 {
        self.0
    }

    pub(crate) fn from_raw(n: i64) -> Option<Self> {
        (n >= 1).then_some(Self(n))
    }
}

impl std::fmt::Display for EntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryRef {
    pub schema: SchemaName,
    pub id: EntryId,
}

impl EntryRef {
    pub fn parse(s: &str) -> Result<Self, Error> {
        let Some((schema, id)) = s.rsplit_once('/') else {
            return Err(Error::Usage(Usage::InvalidLinkTarget(s.to_string())));
        };
        let schema = SchemaName::parse(schema)
            .map_err(|_| Error::Usage(Usage::InvalidLinkTarget(s.to_string())))?;
        let id: i64 = id
            .parse()
            .map_err(|_| Error::Usage(Usage::InvalidLinkTarget(s.to_string())))?;
        let id = EntryId::parse(id)
            .map_err(|_| Error::Usage(Usage::InvalidLinkTarget(s.to_string())))?;
        Ok(Self { schema, id })
    }
}

impl std::fmt::Display for EntryRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.schema, self.id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub name: LinkName,
    pub to: EntryRef,
}

impl Link {
    pub fn parse(name: &str, target: &str) -> Result<Self, Error> {
        Ok(Self {
            name: LinkName::parse(name)?,
            to: EntryRef::parse(target)?,
        })
    }
}

pub fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub fn is_schema_name(s: &str) -> bool {
    !s.is_empty()
        && s.split('.')
            .all(|seg| is_identifier(seg) && !seg.contains('_'))
}

pub fn is_reserved(s: &str) -> bool {
    matches!(s, "id" | "at" | "agent" | "ignored" | "links")
}

pub fn is_time_group(s: &str) -> bool {
    matches!(s, "day" | "week" | "month" | "year")
}

pub fn parse_number(raw: &str) -> Result<Decimal, Error> {
    if raw.contains(['e', 'E']) {
        return Err(Error::Fail(Fail::InvalidNumber(raw.to_string())));
    }
    let Ok(n) = raw.parse() else {
        return Err(Error::Fail(Fail::InvalidNumber(raw.to_string())));
    };
    Ok(n)
}
