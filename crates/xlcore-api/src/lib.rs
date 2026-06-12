mod auto_filter;
mod cells;
mod charts;
mod comments;
mod conditional_format;
mod copyfill;
mod data_validation;
mod defined_names;
mod dependencies;
mod errors;
mod hyperlinks;
mod images;
mod merges;
mod ooxml_header;
mod page_setup;
mod pivots;
mod properties;
mod protection;
mod ranges;
mod raw_parts;
mod refs;
mod rowcols;
mod search;
mod shapes;
mod sheets;
mod sparklines;
mod structural;
mod styles;
mod tables;
mod threaded_notes;
mod vml_comments;
mod worksheet_props;
mod xml;

use std::path::Path;

pub use xlcore_types::{
    AlignmentPatch, AnchorSpec, ApiCellValue, ApiCellValue as CellValue, ApiError, ApiErrorCode,
    ApiWarning, AutoFilterColumnInfo, AutoFilterColumnPatch, AutoFilterCriteria,
    AutoFilterCustomCriterion, AutoFilterInfo, AutoFilterOperator, BorderLinePatch,
    BorderLineStyle, BorderPatch, CalcMode, CalcProperties, CalcPropertiesPatch, CellInfo,
    CfIconSetKind, CfOperator, CfRuleKind, CfValueObject, CfValueObjectKind, ChartAnchor,
    ChartAxisGroup, ChartAxisPatch, ChartInfo, ChartKind, ChartLegendPosition, ChartLine,
    ChartMarker, ChartPatch, ChartSeriesInfo, ChartSeriesPatch, ChartStacking, ChartUpdate,
    ClearMode, ColorScalePatch, CommentInfo, CommentPatch, ConditionalFormatRuleInfo,
    ConditionalFormatRulePatch, CrossBetween, DataBarPatch, DataValidationErrorStyle,
    DataValidationInfo, DataValidationOperator, DataValidationPatch, DataValidationType,
    DefinedNameInfo, DefinedNamePatch, DependencyInfo, DependencyReference, DispBlanksAs,
    FillPatch, FontPatch, FontScheme, FreezeInfo, GradientFillPatch, GradientStopPatch,
    GradientType, HeaderFooterInfo, HeaderFooterPatch, HorizontalAlign, HyperlinkInfo,
    HyperlinkPatch, IconSetPatch, ImageFormat, ImageInfo, ImagePatch, LayoutOptions, LineDash,
    MarkerStyle, MergeInfo, PageMarginsInfo, PageMarginsPatch, PageOrder, PageOrientation,
    PageSetupSettings, PageSetupSettingsPatch, PatternType, PivotAggregation, PivotCellRole,
    PivotDataField, PivotFieldFilter, PivotGrid, PivotGridCell, PivotInfo, PivotPatch, PivotUpdate,
    PrintCellComments, PrintErrors, PrintOptionsInfo, PrintOptionsPatch, ProtectionPatch,
    RadarStyle, RangeInfo, ReadingOrder, SearchHit, SearchMatch, SearchMode, SearchOptions,
    SearchTarget, ShapeInfo, ShapeLineEnd, ShapePatch, SheetInfo, SheetPageSetup,
    SheetPageSetupPatch, SheetProperties, SheetPropertiesPatch, SheetProtectionInfo,
    SheetProtectionPatch, SheetVisibility, SparklineAxisType, SparklineDisplayBlanks,
    SparklineEntry, SparklineGroupInfo, SparklineGroupPatch, SparklineKind, StylePatch,
    TableColumnInfo, TableColumnPatch, TableInfo, TablePatch, TableStylePatch, TableStyleSettings,
    TableTotalsFunction, ThreadedNoteInfo, ThreadedNotePatch, TickLabelPosition, TickMark,
    UnderlinePatch, VertAlign, VerticalAlign, WorkbookProperties, WorkbookPropertiesPatch,
    WorkbookProtectionInfo, WorkbookProtectionPatch,
};

use crate::errors::{anyhow_err_to_api, load_err_to_api};
use crate::xml::blank_workbook_bytes;

pub type Result<T> = std::result::Result<T, ApiError>;

pub struct Workbook {
    pub(crate) doc: xlcore_io::SpreadsheetDocument,
    pub(crate) report: xlcore_io::LoadReport,
    pub(crate) warnings: Vec<ApiWarning>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BatchOutcome<T> {
    pub value: Option<T>,
    pub warnings: Vec<ApiWarning>,
    pub error: Option<ApiError>,
}

impl<T> BatchOutcome<T> {
    pub fn is_ok(&self) -> bool {
        self.error.is_none()
    }

    pub fn into_result(self) -> Result<(T, Vec<ApiWarning>)> {
        match (self.value, self.error) {
            (Some(v), None) => Ok((v, self.warnings)),
            (_, Some(e)) => Err(e),
            (None, None) => Err(ApiError::new(
                ApiErrorCode::Other,
                "batch produced no value and no error",
            )),
        }
    }
}

impl Workbook {
    pub fn new() -> Result<Self> {
        let (doc, report) =
            xlcore_io::open_bytes_with_report(blank_workbook_bytes()?).map_err(load_err_to_api)?;
        Ok(Self {
            doc,
            report,
            warnings: Vec::new(),
        })
    }

    pub fn open_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self> {
        let (doc, report) =
            xlcore_io::open_bytes_with_report(bytes.into()).map_err(load_err_to_api)?;
        Ok(Self {
            doc,
            report,
            warnings: Vec::new(),
        })
    }

    pub fn open_path(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path)
            .map_err(|err| ApiError::new(ApiErrorCode::Other, err.to_string()))?;
        Self::open_bytes(bytes)
    }

    pub fn load_report(&self) -> &xlcore_io::LoadReport {
        &self.report
    }

    pub fn save_bytes(&mut self) -> Result<Vec<u8>> {
        let _ = self.recalculate(false)?;
        self.doc
            .to_package_bytes()
            .map_err(|err| ApiError::new(ApiErrorCode::OoxmlWriteError, err.to_string()))
    }

    pub fn save_path(&mut self, path: impl AsRef<Path>) -> Result<()> {
        std::fs::write(path, self.save_bytes()?)
            .map_err(|err| ApiError::new(ApiErrorCode::OoxmlWriteError, err.to_string()))
    }

    pub fn warnings(&self) -> &[ApiWarning] {
        &self.warnings
    }

    pub fn take_warnings(&mut self) -> Vec<ApiWarning> {
        std::mem::take(&mut self.warnings)
    }

    #[allow(dead_code)]
    pub(crate) fn push_warning(&mut self, warning: ApiWarning) {
        self.warnings.push(warning);
    }

    pub fn batch<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> BatchOutcome<T> {
        let prior = std::mem::take(&mut self.warnings);
        let outcome = f(self);
        let warnings = std::mem::take(&mut self.warnings);
        self.warnings = prior;
        match outcome {
            Ok(value) => BatchOutcome {
                value: Some(value),
                warnings,
                error: None,
            },
            Err(error) => BatchOutcome {
                value: None,
                warnings,
                error: Some(error),
            },
        }
    }

    pub fn recalculate(&mut self, errors_only: bool) -> Result<xlcore_bridge::RecalcWorkbook> {
        let mut report = xlcore_bridge::recalculate_doc_with_writeback(&mut self.doc)
            .map_err(anyhow_err_to_api)?;
        if errors_only {
            for sheet in &mut report.sheets {
                sheet.cells.retain(|cell| cell.fallback.is_some());
            }
            report.sheets.retain(|sheet| !sheet.cells.is_empty());
        }
        Ok(report)
    }

    pub fn layout(&mut self, options: LayoutOptions) -> Result<xlcore_export::WorkbookLayout> {
        let options = xlcore_export::ExtractOptions {
            sheet_index: options.sheet_index,
            sheet_name: options.sheet_name,
        };
        xlcore_export::extract_doc_with_options(&mut self.doc, &options).map_err(anyhow_err_to_api)
    }

    pub fn recalculate_layout(
        &mut self,
        options: LayoutOptions,
    ) -> Result<(xlcore_bridge::RecalcWorkbook, xlcore_export::WorkbookLayout)> {
        let recalculated = self.recalculate(false)?;
        let layout = self.layout(options)?;
        Ok((recalculated, layout))
    }
}

#[cfg(test)]
mod tests;
