use crate::StylePatch;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct AutoFilterInfo {
    pub sheet: String,
    pub reference: String,
    pub start_row: u32,
    pub start_column: u32,
    pub end_row: u32,
    pub end_column: u32,
    #[serde(default)]
    pub columns: Vec<AutoFilterColumnInfo>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct AutoFilterColumnInfo {
    pub column_offset: u32,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden_button: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_button: Option<bool>,
    pub criteria: AutoFilterCriteria,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AutoFilterCriteria {
    Values {
        #[serde(default)]
        values: Vec<String>,
        #[serde(default)]
        blank: bool,
    },
    Top10 {
        #[serde(default)]
        top: bool,
        #[serde(default)]
        percent: bool,
        val: f64,
    },
    Custom {
        #[serde(default)]
        logical_and: bool,
        #[serde(default)]
        criteria: Vec<AutoFilterCustomCriterion>,
    },
    Unsupported {
        name: String,
    },
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct AutoFilterCustomCriterion {
    pub operator: AutoFilterOperator,
    pub value: String,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum AutoFilterOperator {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct AutoFilterColumnPatch {
    pub column_offset: u32,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden_button: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_button: Option<bool>,
    pub criteria: AutoFilterCriteria,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum DataValidationType {
    #[default]
    List,
    Custom,
    Whole,
    Decimal,
    Date,
    Time,
    TextLength,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum DataValidationOperator {
    Between,
    NotBetween,
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum DataValidationErrorStyle {
    Stop,
    Warning,
    Information,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct DataValidationInfo {
    pub sheet: String,
    pub reference: String,
    pub ranges: Vec<String>,
    pub rule_type: DataValidationType,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<DataValidationOperator>,
    pub allow_blank: bool,
    pub show_drop_down: bool,
    pub show_input_message: bool,
    pub show_error_message: bool,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_style: Option<DataValidationErrorStyle>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula1: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula2: Option<String>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct DataValidationPatch {
    pub rule_type: DataValidationType,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<DataValidationOperator>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_blank: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_drop_down: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_input_message: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_error_message: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_style: Option<DataValidationErrorStyle>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_title: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula1: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula2: Option<String>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum CfRuleKind {
    #[default]
    Expression,
    CellIs,
    Top10,
    DuplicateValues,
    UniqueValues,
    ContainsText,
    NotContainsText,
    BeginsWith,
    EndsWith,
    ContainsBlanks,
    NotContainsBlanks,
    ContainsErrors,
    NotContainsErrors,
    TimePeriod,
    AboveAverage,
    ColorScale,
    DataBar,
    IconSet,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum CfValueObjectKind {
    #[default]
    Number,
    Percent,
    Max,
    Min,
    Formula,
    Percentile,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct CfValueObject {
    pub kind: CfValueObjectKind,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct ColorScalePatch {
    pub values: Vec<CfValueObject>,
    pub colors: Vec<String>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct DataBarPatch {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<CfValueObject>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<CfValueObject>,
    pub color: String,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_value: Option<bool>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum CfIconSetKind {
    #[default]
    ThreeArrows,
    ThreeArrowsGray,
    ThreeFlags,
    ThreeTrafficLights1,
    ThreeTrafficLights2,
    ThreeSigns,
    ThreeSymbols,
    ThreeSymbols2,
    FourArrows,
    FourArrowsGray,
    FourRedToBlack,
    FourRating,
    FourTrafficLights,
    FiveArrows,
    FiveArrowsGray,
    FiveRating,
    FiveQuarters,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct IconSetPatch {
    pub icon_set: CfIconSetKind,
    pub values: Vec<CfValueObject>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_value: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverse: Option<bool>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub enum CfOperator {
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
    GreaterThanOrEqual,
    GreaterThan,
    Between,
    NotBetween,
    ContainsText,
    NotContains,
    BeginsWith,
    EndsWith,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalFormatRulePatch {
    pub kind: CfRuleKind,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<CfOperator>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula1: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula2: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub above_average: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equal_average: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub std_dev: Option<i32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_if_true: Option<bool>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dxf: Option<StylePatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_scale: Option<ColorScalePatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_bar: Option<DataBarPatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_set: Option<IconSetPatch>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalFormatRuleInfo {
    pub sheet: String,
    pub ranges: Vec<String>,
    pub kind: CfRuleKind,
    pub priority: i32,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<CfOperator>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula1: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula2: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,
    #[serde(default)]
    pub percent: bool,
    #[serde(default)]
    pub bottom: bool,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub above_average: Option<bool>,
    #[serde(default)]
    pub equal_average: bool,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub std_dev: Option<i32>,
    #[serde(default)]
    pub stop_if_true: bool,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dxf_id: Option<u32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_scale: Option<ColorScalePatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_bar: Option<DataBarPatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_set: Option<IconSetPatch>,
}
