use ooxmlsdk::common::SdkError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaErrorKind {
    InvalidFieldValue,

    InvalidEnumValue,

    Validation,

    UnexpectedTag,

    MissingField,

    UnexpectedEof,

    Other,
}

impl SchemaErrorKind {
    /// Stable PascalCase identifier shared across Rust (Debug-equivalent),
    /// the wasm error payload, and the TypeScript `XlsxSchemaErrorKind` union.
    pub fn as_str(&self) -> &'static str {
        match self {
            SchemaErrorKind::InvalidFieldValue => "InvalidFieldValue",
            SchemaErrorKind::InvalidEnumValue => "InvalidEnumValue",
            SchemaErrorKind::Validation => "Validation",
            SchemaErrorKind::UnexpectedTag => "UnexpectedTag",
            SchemaErrorKind::MissingField => "MissingField",
            SchemaErrorKind::UnexpectedEof => "UnexpectedEof",
            SchemaErrorKind::Other => "Other",
        }
    }
}

impl serde::Serialize for SchemaErrorKind {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

#[derive(Debug, Error)]
pub enum XlsxLoadError {
    #[error("not a valid xlsx archive: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("schema error in {part}: {kind:?}{}",
        match (.field.as_deref(), .value.as_deref()) {
            (Some(f), Some(v)) => format!(" ({}={:?})", f, v),
            (Some(f), None) => format!(" ({})", f),
            (None, Some(v)) => format!(" (value {:?})", v),
            (None, None) => String::new(),
        })]
    Schema {
        part: String,
        kind: SchemaErrorKind,

        ty: Option<String>,

        field: Option<String>,

        value: Option<String>,

        message: String,
    },

    #[error("missing required part: {part}")]
    MissingPart { part: &'static str },

    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

// NOTE: ooxmlsdk's `SdkError` does not carry the originating package part /
// zip path, so SDK-originated schema errors land here with `part =
// "<sdk-internal>"`. Errors detected by the precompile fixer pipeline (before
// the SDK runs) carry an accurate `part`. To get accurate parts for SDK
// errors we'd need an upstream change to ooxmlsdk or a per-part loader.
pub(crate) const SDK_UNKNOWN_PART: &str = "<sdk-internal>";

pub(crate) fn map_sdk_error(err: SdkError) -> XlsxLoadError {
    match err {
        SdkError::ZipError(zip_err) => XlsxLoadError::Zip(zip_err),
        SdkError::StdIoError(io) => XlsxLoadError::Io(io),
        SdkError::InvalidFieldValue { ty, field, value } => XlsxLoadError::Schema {
            part: SDK_UNKNOWN_PART.into(),
            kind: SchemaErrorKind::InvalidFieldValue,
            ty: Some(ty.to_string()),
            field: Some(field.to_string()),
            value: Some(value),
            message: String::new(),
        },
        SdkError::InvalidEnumValue { ty, value } => XlsxLoadError::Schema {
            part: SDK_UNKNOWN_PART.into(),
            kind: SchemaErrorKind::InvalidEnumValue,
            ty: Some(ty.to_string()),
            field: None,
            value: Some(value),
            message: String::new(),
        },
        SdkError::ValidationError {
            ty,
            field,
            value,
            message,
            ..
        } => XlsxLoadError::Schema {
            part: SDK_UNKNOWN_PART.into(),
            kind: SchemaErrorKind::Validation,
            ty: Some(ty.to_string()),
            field: Some(field.to_string()),
            value: Some(value),
            message,
        },
        SdkError::UnexpectedTag {
            ty,
            expected,
            found,
        } => XlsxLoadError::Schema {
            part: SDK_UNKNOWN_PART.into(),
            kind: SchemaErrorKind::UnexpectedTag,
            ty: Some(ty.to_string()),
            field: None,
            value: Some(found),
            message: format!("expected {expected}"),
        },
        SdkError::MissingField { ty, field } => XlsxLoadError::Schema {
            part: SDK_UNKNOWN_PART.into(),
            kind: SchemaErrorKind::MissingField,
            ty: Some(ty.to_string()),
            field: Some(field.to_string()),
            value: None,
            message: String::new(),
        },
        SdkError::UnexpectedEof { context } => XlsxLoadError::Schema {
            part: context.to_string(),
            kind: SchemaErrorKind::UnexpectedEof,
            ty: None,
            field: None,
            value: None,
            message: format!("unexpected EOF while parsing {context}"),
        },
        other => XlsxLoadError::Other(other.to_string()),
    }
}

impl XlsxLoadError {
    pub fn code(&self) -> &'static str {
        match self {
            XlsxLoadError::Zip(_) => "Zip",
            XlsxLoadError::Schema { .. } => "Schema",
            XlsxLoadError::MissingPart { .. } => "MissingPart",
            XlsxLoadError::Io(_) => "Io",
            XlsxLoadError::Other(_) => "Other",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FixedAttribute {
    pub part: String,

    pub ty: Option<String>,

    pub field: Option<String>,

    pub value: Option<String>,

    pub occurrences: usize,

    pub kind: SchemaErrorKind,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadReport {
    pub fixes: Vec<FixedAttribute>,

    pub warnings: Vec<String>,
}

impl LoadReport {
    pub fn is_clean(&self) -> bool {
        self.fixes.is_empty() && self.warnings.is_empty()
    }
}
