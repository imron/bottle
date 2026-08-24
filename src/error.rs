use crate::spec::{EntryRef, EnumValue, FieldName, Ident, LinkName, SchemaName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Usage(Usage),
    Fail(Fail),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Usage {
    EnumValuesRequired,
    EnumValuesNotAllowed,
    AmendEmpty,
    DuplicateUnlink(LinkName),
    LinkAndUnlink(LinkName),
    ReservedWhere(Ident),
    DuplicateLinkName(LinkName),
    DuplicateField(FieldName),
    InvalidLinkTarget(String),
    DateOnlyNotInstant,
    TimeMustUseT,
    InvalidDate(String),
    InvalidTime(String),
    OffsetNeedsColon,
    UnknownHelpTopic(String),
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
    EntryNotFound { schema: SchemaName, id: i64 },
    NotFound,
    FieldNotNumber(FieldName),
    LinkNameCollidesWithField(LinkName),
    LinkTargetMissing(EntryRef),
    MissingRequiredField(FieldName),
    TextHasTabOrNewline(FieldName),
    EnumHasNoValues(FieldName),
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
    ValuesOnlyForEnum(FieldName),
    EnumNeedsValues(FieldName),
    InvalidIdent(String),
    CorruptSchemaName(String),
    CorruptLinkName(String),
    CorruptLinkSchema(String),
    CorruptStoredTime(String),
    HomeNotSet,
    DbPathRequired,
    HelpNotAnOp,
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

    pub fn message(&self) -> String {
        self.to_string()
    }
}

impl std::error::Error for Error {}

pub(crate) trait UniqueConstraint<T> {
    fn unique(self, err: Fail) -> Result<T, Error>;
}

impl<T> UniqueConstraint<T> for Result<T, rusqlite::Error> {
    fn unique(self, err: Fail) -> Result<T, Error> {
        self.map_err(|e| {
            if e.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
                Error::Fail(err)
            } else {
                Error::from(e)
            }
        })
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Self::Fail(Fail::Store(err.to_string()))
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Fail(Fail::Io(err.to_string()))
    }
}

impl From<rust_decimal::Error> for Error {
    fn from(err: rust_decimal::Error) -> Self {
        Self::Fail(Fail::InvalidNumber(err.to_string()))
    }
}

impl From<Error> for rusqlite::Error {
    fn from(err: Error) -> Self {
        rusqlite::Error::UserFunctionError(Box::new(err))
    }
}
