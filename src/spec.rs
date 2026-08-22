use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: FieldType,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
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
            serde_yaml::from_str(raw).map_err(|e| Error::fail(format!("invalid spec: {e}")))?;
        spec.canonicalize()?;
        Ok(spec)
    }

    pub fn to_yaml(&self) -> Result<String, Error> {
        serde_yaml::to_string(self).map_err(|e| Error::fail(e.to_string()))
    }

    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    fn canonicalize(&mut self) -> Result<(), Error> {
        let mut seen = std::collections::HashSet::new();
        for field in &mut self.fields {
            if !is_ident(&field.name) {
                return Err(Error::fail(format!("invalid field name: {}", field.name)));
            }
            if is_reserved(&field.name) {
                return Err(Error::fail(format!("reserved field name: {}", field.name)));
            }
            if !seen.insert(field.name.clone()) {
                return Err(Error::fail(format!("duplicate field: {}", field.name)));
            }
            match field.type_ {
                FieldType::Enum => {
                    let Some(values) = field.values.as_mut() else {
                        return Err(Error::fail(format!("enum {} needs values", field.name)));
                    };
                    fold_enum_values(values)?;
                }
                _ => {
                    if field.values.is_some() {
                        return Err(Error::fail(format!(
                            "values only apply to enum, not {}",
                            field.name
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub fn is_schema_name(s: &str) -> bool {
    let mut parts = s.split('.');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(a), Some(b), None) if is_ident(a) && is_ident(b)
    )
}

pub fn is_reserved(s: &str) -> bool {
    matches!(s, "id" | "at" | "agent" | "ignored" | "links")
}

pub fn is_time_group(s: &str) -> bool {
    matches!(s, "day" | "week" | "month" | "year")
}

pub fn table_name(schema: &str) -> String {
    schema.replace('.', "_")
}

pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

pub fn fold_enum(value: &str) -> String {
    value.to_ascii_lowercase()
}

pub fn fold_enum_values(values: &mut [String]) -> Result<(), Error> {
    let mut seen = std::collections::HashSet::new();
    for value in values.iter_mut() {
        *value = fold_enum(value);
        if value.is_empty() {
            return Err(Error::fail("empty enum value"));
        }
        if !seen.insert(value.clone()) {
            return Err(Error::fail(format!(
                "duplicate enum value after fold: {value}"
            )));
        }
    }
    Ok(())
}

pub fn parse_target(s: &str) -> Result<(String, i64), Error> {
    let Some((schema, id)) = s.rsplit_once('/') else {
        return Err(Error::usage(format!("invalid link target: {s}")));
    };
    if !is_schema_name(schema) {
        return Err(Error::usage(format!("invalid link target: {s}")));
    }
    let id: i64 = id
        .parse()
        .map_err(|_| Error::usage(format!("invalid link target: {s}")))?;
    Ok((schema.to_string(), id))
}

pub fn parse_number(raw: &str) -> Result<f64, Error> {
    if raw.contains(['e', 'E']) {
        return Err(Error::fail(format!("invalid number: {raw}")));
    }
    let n: f64 = raw
        .parse()
        .map_err(|_| Error::fail(format!("invalid number: {raw}")))?;
    if !n.is_finite() {
        return Err(Error::fail(format!("invalid number: {raw}")));
    }
    Ok(n)
}
