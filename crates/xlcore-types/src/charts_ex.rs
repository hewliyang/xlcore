use serde::{Deserialize, Serialize};

use crate::charts::{AnchorSpec, ChartAnchor, ChartLegendPosition};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// A modern "chartEx" chart layout (`cx:series/@layoutId`, OOXML `ST_SeriesLayout`),
/// transliterated from ooxmlsdk `SeriesLayout`. These charts live in a separate
/// `chartEx{N}.xml` part (`cx:` namespace) referenced from the drawing, distinct
/// from the legacy `c:` charts authored via {@link ChartPatch}.
///
/// `histogram` and `pareto` are authored as `clusteredColumn` / `paretoLine`
/// series under the hood; this enum collapses those to the user-facing layout.
/// All eight are renderer-visible in xlsx-preview.
pub enum ChartExKind {
    #[default]
    Waterfall,
    Funnel,
    Treemap,
    Sunburst,
    Histogram,
    Pareto,
    #[serde(rename = "boxWhisker")]
    BoxWhisker,
    #[serde(rename = "regionMap")]
    RegionMap,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
/// Box-and-whisker quartile calculation method (`cx:statistics/@quartileMethod`,
/// OOXML `ST_QuartileMethod`), transliterated from ooxmlsdk `QuartileMethod`.
/// Round-trips for Excel; the xlsx-preview renderer always uses inclusive quartiles.
pub enum ChartExQuartileMethod {
    #[default]
    #[serde(rename = "inclusive")]
    Inclusive,
    #[serde(rename = "exclusive")]
    Exclusive,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
/// A chartEx data series (`cx:series`), distilled from ooxmlsdk `Series`.
///
/// `values_ref` is the `cx:numDim` formula (typically a sheet range like
/// `Sheet1!$B$2:$B$7`); its dimension type is derived from the parent chart's
/// kind (`val` for waterfall/funnel/histogram/pareto/boxWhisker, `size` for
/// treemap/sunburst, `colorVal` for regionMap). `name`/`name_ref` populate
/// `cx:tx`. Most kinds take a single series; `boxWhisker` accepts several.
///
/// schema-excluded: spPr, valueColors, valueColorPositions, dataPt, dataLabels,
/// hidden, ownerIdx, uniqueId, formatIdx, axisId, extLst
pub struct ChartExSeriesPatch {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_ref: Option<String>,
    pub values_ref: String,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
/// Authoring patch for a modern chartEx chart (`cx:chartSpace` in a separate
/// `chartEx{N}.xml` part), distilled from ooxmlsdk `ChartSpace`/`Chart`/`Series`.
///
/// Covers the eight `ChartExKind` layouts the xlsx-preview renderer draws.
/// `categories_ref` is the `cx:strDim type="cat"` formula; for treemap/sunburst
/// it may be a multi-column range whose columns become hierarchy levels.
/// Type-specific knobs (`subtotals`, `bin_count`/`bin_size`, `quartile_method`)
/// apply only to their relevant kind and are ignored otherwise.
///
/// schema-excluded: spPr, txPr, clrMapOvr, fmtOvrs, printSettings, externalData,
/// plotSurface, dataLabels, valueColors, dataPt, axis styling, extLst
pub struct ChartExPatch {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub kind: ChartExKind,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default)]
    pub anchor: AnchorSpec,
    /// `cx:strDim type="cat"` formula (e.g. `Sheet1!$A$2:$A$7`). Optional for
    /// `histogram` (raw observations need no categories).
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories_ref: Option<String>,
    pub series: Vec<ChartExSeriesPatch>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend_position: Option<ChartLegendPosition>,
    /// `cx:subtotals`; 0-based indices of points drawn as totals from zero.
    /// Waterfall only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subtotals: Vec<u32>,
    /// `cx:binning/cx:binCount`; number of histogram bins. Histogram only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bin_count: Option<u32>,
    /// `cx:binning/cx:binSize`; histogram bin width. Histogram only. Ignored when
    /// `bin_count` is set.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bin_size: Option<f64>,
    /// `cx:statistics/@quartileMethod`. Box-and-whisker only.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quartile_method: Option<ChartExQuartileMethod>,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
/// A chartEx series as read back (`cx:series`).
pub struct ChartExSeriesInfo {
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_ref: Option<String>,
    pub values_ref: String,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../../packages/xlsx-preview/src/api-schema/")
)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
/// A modern chartEx chart as read back from `chartEx{N}.xml`.
pub struct ChartExInfo {
    pub sheet: String,
    pub id: String,
    pub name: String,
    pub kind: ChartExKind,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub anchor: ChartAnchor,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories_ref: Option<String>,
    pub series: Vec<ChartExSeriesInfo>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend_position: Option<ChartLegendPosition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subtotals: Vec<u32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bin_count: Option<u32>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bin_size: Option<f64>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quartile_method: Option<ChartExQuartileMethod>,
}
