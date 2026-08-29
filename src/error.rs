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
    HomeNotSet,
    FileNotFound(String),
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
