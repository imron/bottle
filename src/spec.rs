use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Fail, Usage};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: FieldName,
    #[serde(rename = "type")]
    pub type_: FieldType,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<EnumValue>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Number,
    Enum,
}

impl Spec {
    pub fn parse_yaml(raw: &str) -> Result<Self, Error> {
        let mut spec: Spec =
            serde_yaml::from_str(raw).map_err(|e| Error::Fail(Fail::InvalidSpec(e.to_string())))?;
        spec.canonicalize()?;
        Ok(spec)
    }

    pub fn to_yaml(&self) -> Result<String, Error> {
        serde_yaml::to_string(self).map_err(|e| Error::Fail(Fail::Yaml(e.to_string())))
    }

    pub fn field(&self, name: &FieldName) -> Option<&Field> {
        self.fields.iter().find(|f| &f.name == name)
    }

    fn canonicalize(&mut self) -> Result<(), Error> {
        let mut seen = std::collections::HashSet::new();
        for field in &mut self.fields {
            if !seen.insert(field.name.clone()) {
                return Err(Error::Fail(Fail::DuplicateSpecField(field.name.clone())));
            }
            match field.type_ {
                FieldType::Enum => {
                    let Some(values) = field.values.as_mut() else {
                        return Err(Error::Fail(Fail::EnumNeedsValues(field.name.clone())));
                    };
                    let mut seen_values = std::collections::HashSet::new();
                    for value in values.iter() {
                        if !seen_values.insert(value.clone()) {
                            return Err(Error::Fail(Fail::DuplicateEnumValue(value.clone())));
                        }
                    }
                }
                _ => {
                    if field.values.is_some() {
                        return Err(Error::Fail(Fail::ValuesOnlyForEnum(field.name.clone())));
                    }
                }
            }
        }
        Ok(())
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

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaName(String);

impl SchemaName {
    pub fn parse(s: &str) -> Result<Self, Error> {
        if is_schema_name(s) {
            Ok(Self(s.to_string()))
        } else {
            Err(Error::Fail(Fail::InvalidSchemaName(s.to_string())))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SchemaName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for SchemaName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

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

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for FieldName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Serialize for FieldName {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for FieldName {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        FieldName::parse(&raw).map_err(serde::de::Error::custom)
    }
}

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

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for LinkName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for LinkName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EnumValue(String);

impl EnumValue {
    pub fn parse(s: &str) -> Result<Self, Error> {
        let folded = s.to_ascii_lowercase();
        if folded.is_empty() {
            return Err(Error::Fail(Fail::EmptyEnumValue));
        }
        Ok(Self(folded))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EnumValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for EnumValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for EnumValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        EnumValue::parse(&raw).map_err(serde::de::Error::custom)
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryRef {
    pub schema: SchemaName,
    pub id: i64,
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
        if id < 1 {
            return Err(Error::Usage(Usage::InvalidLinkTarget(s.to_string())));
        }
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
    !s.is_empty() && s.split('.').all(is_identifier)
}

pub fn is_reserved(s: &str) -> bool {
    matches!(s, "id" | "at" | "agent" | "ignored" | "links")
}

pub fn is_time_group(s: &str) -> bool {
    matches!(s, "day" | "week" | "month" | "year")
}

pub fn fold_enum(value: &str) -> Result<EnumValue, Error> {
    EnumValue::parse(value)
}

pub fn fold_enum_values(values: Vec<String>) -> Result<Vec<EnumValue>, Error> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for value in values {
        let folded = EnumValue::parse(&value)?;
        if !seen.insert(folded.clone()) {
            return Err(Error::Fail(Fail::DuplicateEnumValue(folded)));
        }
        out.push(folded);
    }
    Ok(out)
}

pub fn parse_number(raw: &str) -> Result<Decimal, Error> {
    if raw.contains(['e', 'E']) {
        return Err(Error::Fail(Fail::InvalidNumber(raw.to_string())));
    }
    Ok(raw.parse()?)
}
