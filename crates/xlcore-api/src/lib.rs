mod cells;
mod copyfill;
mod errors;
mod merges;
mod ranges;
mod refs;
mod rowcols;
mod search;
mod sheets;
mod structural;
mod styles;
mod xml;

use std::path::Path;

pub use xlcore_types::{
    AlignmentPatch, ApiCellValue, ApiCellValue as CellValue, ApiError, ApiErrorCode,
    BorderLinePatch, BorderLineStyle, BorderPatch, CellInfo, ClearMode, FillPatch, FontPatch,
    FreezeInfo, HorizontalAlign, LayoutOptions, MergeInfo, RangeInfo, SearchHit, SearchMatch,
    SearchMode, SearchOptions, SearchTarget, SheetInfo, SheetVisibility, StylePatch,
    UnderlinePatch, VerticalAlign,
};

use crate::errors::{anyhow_err_to_api, load_err_to_api};
use crate::xml::blank_workbook_bytes;

pub type Result<T> = std::result::Result<T, ApiError>;

pub struct Workbook {
    pub(crate) doc: xlcore_io::SpreadsheetDocument,
    pub(crate) report: xlcore_io::LoadReport,
}

impl Workbook {
    pub fn new() -> Result<Self> {
        let (doc, report) =
            xlcore_io::open_bytes_with_report(blank_workbook_bytes()?).map_err(load_err_to_api)?;
        Ok(Self { doc, report })
    }

    pub fn open_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self> {
        let (doc, report) =
            xlcore_io::open_bytes_with_report(bytes.into()).map_err(load_err_to_api)?;
        Ok(Self { doc, report })
    }

    pub fn open_path(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path)
            .map_err(|err| ApiError::new(ApiErrorCode::Other, err.to_string()))?;
        Self::open_bytes(bytes)
    }

    pub fn load_report(&self) -> &xlcore_io::LoadReport {
        &self.report
    }

    pub fn save_bytes(&self) -> Result<Vec<u8>> {
        self.doc
            .to_package_bytes()
            .map_err(|err| ApiError::new(ApiErrorCode::OoxmlWriteError, err.to_string()))
    }

    pub fn save_path(&self, path: impl AsRef<Path>) -> Result<()> {
        std::fs::write(path, self.save_bytes()?)
            .map_err(|err| ApiError::new(ApiErrorCode::OoxmlWriteError, err.to_string()))
    }

    pub fn batch<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        f(self)
    }

    pub fn recalculate(&mut self) -> Result<xlcore_bridge::RecalcWorkbook> {
        xlcore_bridge::recalculate_doc_with_writeback(&mut self.doc).map_err(anyhow_err_to_api)
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
        let recalculated = self.recalculate()?;
        let layout = self.layout(options)?;
        Ok((recalculated, layout))
    }
}

#[cfg(test)]
mod tests;
