use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    InvalidRef,
    MissingSheet,
    DuplicateSheet,
    InvalidSheetName,
    CannotDeleteLastSheet,
    ShapeMismatch,
    UnsupportedStyle,
    MergeOverlap,
    InvalidHyperlink,
    InvalidComment,
    InvalidThreadedNote,
    InvalidDataValidation,
    InvalidSearchQuery,
    InvalidDefinedName,
    InvalidProperty,
    InvalidTable,
    DuplicateTable,
    InvalidProtection,
    InvalidPageSetup,
    InvalidAutoFilter,
    InvalidConditionalFormat,
    InvalidChart,
    InvalidImage,
    InvalidShape,
    InvalidSparklineGroup,
    InvalidPivot,
    UnsupportedFormula,
    UnsupportedObject,
    LossyOperation,
    OoxmlWriteError,
    Other,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    pub fn new(code: ApiErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            sheet: None,
            reference: None,
            part: None,
        }
    }

    pub fn with_sheet(mut self, sheet: impl Into<String>) -> Self {
        self.sheet = Some(sheet.into());
        self
    }

    pub fn with_ref(mut self, reference: impl Into<String>) -> Self {
        self.reference = Some(reference.into());
        self
    }

    pub fn with_part(mut self, part: impl Into<String>) -> Self {
        self.part = Some(part.into());
        self
    }
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct ApiWarning {
    pub code: ApiErrorCode,
    pub message: String,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
}

impl ApiWarning {
    pub fn new(code: ApiErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            sheet: None,
            reference: None,
            part: None,
        }
    }

    pub fn with_sheet(mut self, sheet: impl Into<String>) -> Self {
        self.sheet = Some(sheet.into());
        self
    }

    pub fn with_ref(mut self, reference: impl Into<String>) -> Self {
        self.reference = Some(reference.into());
        self
    }

    pub fn with_part(mut self, part: impl Into<String>) -> Self {
        self.part = Some(part.into());
        self
    }
}
