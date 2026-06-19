use std::collections::HashMap;

use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::sdk::SdkPart;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{ApiError, ApiErrorCode, SheetInfo, SheetVisibility};

use crate::errors::sdk_err_to_api;
use crate::refs::validate_sheet_name;
use crate::structural::rename_sheet_in_formula_refs;
use crate::xml::{empty_worksheet, sheet_dimensions, sheet_state_name};
use crate::{Result, Workbook};

impl Workbook {
    pub fn sheets(&mut self) -> Result<Vec<SheetInfo>> {
        let active_tab = self.active_sheet_index()?;
        let sheet_entries = self.workbook_sheets()?;
        let ws_parts = self.worksheet_parts_by_relationship_id()?;
        let mut out = Vec::with_capacity(sheet_entries.len());

        for (index, sheet) in sheet_entries.iter().enumerate() {
            let (row_count, column_count) = ws_parts
                .get(sheet.id.as_str())
                .and_then(|part| sheet_dimensions(&mut self.doc, part).ok())
                .unwrap_or((0, 0));
            out.push(SheetInfo {
                index,
                id: sheet.sheet_id,
                name: sheet.name.as_str().to_string(),
                state: sheet.state.as_ref().and_then(sheet_state_name),
                row_count,
                column_count,
                active: active_tab == Some(index as u32),
            });
        }
        Ok(out)
    }

    pub fn create_sheet(&mut self, name: impl AsRef<str>) -> Result<SheetInfo> {
        let name = validate_sheet_name(name.as_ref())?;
        if self.sheet_exists(name)? {
            return Err(ApiError::new(
                ApiErrorCode::DuplicateSheet,
                format!("sheet already exists: {name}"),
            )
            .with_sheet(name));
        }

        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let ws_part: WorksheetPart = wb_part
            .add_new_part_auto_id(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        ws_part
            .set_root_element(&mut self.doc, empty_worksheet())
            .map_err(sdk_err_to_api)?;

        let relationship_id = ws_part
            .relationship_id()
            .ok_or_else(|| {
                ApiError::new(
                    ApiErrorCode::Other,
                    "new worksheet is missing relationship id",
                )
            })?
            .to_string();

        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let next_sheet_id = workbook
            .sheets
            .sheet
            .iter()
            .map(|sheet| sheet.sheet_id)
            .max()
            .unwrap_or(0)
            + 1;
        workbook.sheets.sheet.push(x::Sheet {
            name: name.to_string(),
            sheet_id: next_sheet_id,
            state: None,
            id: relationship_id,
            ..Default::default()
        });

        let index = workbook.sheets.sheet.len() - 1;
        self.invalidate_engine();
        Ok(SheetInfo {
            index,
            id: next_sheet_id,
            name: name.to_string(),
            state: None,
            row_count: 0,
            column_count: 0,
            active: self.active_sheet_index()? == Some(index as u32),
        })
    }

    pub fn rename_sheet(
        &mut self,
        old_name: impl AsRef<str>,
        new_name: impl AsRef<str>,
    ) -> Result<()> {
        let old_name = old_name.as_ref();
        let new_name = validate_sheet_name(new_name.as_ref())?;
        if old_name != new_name && self.sheet_exists(new_name)? {
            return Err(ApiError::new(
                ApiErrorCode::DuplicateSheet,
                format!("sheet already exists: {new_name}"),
            )
            .with_sheet(new_name));
        }

        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(sheet) = workbook
            .sheets
            .sheet
            .iter_mut()
            .find(|sheet| sheet.name.as_str() == old_name)
        else {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {old_name}"),
            )
            .with_sheet(old_name));
        };
        sheet.name = new_name.to_string();

        let sheet_names: Vec<String> = self
            .workbook_sheets()?
            .iter()
            .map(|s| s.name.as_str().to_string())
            .collect();
        for name in &sheet_names {
            let p = self.worksheet_part_for_sheet(name)?;
            let ws = p.root_element_mut(&mut self.doc).map_err(sdk_err_to_api)?;
            for row in &mut ws.sheet_data.row {
                for cell in &mut row.cell {
                    if let Some(formula) = cell.cell_formula.as_mut() {
                        if let Some(text) = formula.xml_content.as_mut() {
                            *text = rename_sheet_in_formula_refs(text, old_name, new_name);
                        }
                    }
                }
            }
            for cf in &mut ws.conditional_formatting {
                for rule in &mut cf.conditional_formatting_rule {
                    for f in &mut rule.formula {
                        if let Some(text) = f.xml_content.as_mut() {
                            *text = rename_sheet_in_formula_refs(text, old_name, new_name);
                        }
                    }
                }
            }
        }

        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        if let Some(dns) = workbook.defined_names.as_mut() {
            for dn in &mut dns.defined_name {
                if let Some(text) = dn.xml_content.as_mut() {
                    *text = rename_sheet_in_formula_refs(text, old_name, new_name);
                }
            }
        }

        self.mark_formulas_stale()?;
        Ok(())
    }

    pub fn delete_sheet(&mut self, name: impl AsRef<str>) -> Result<()> {
        let name = name.as_ref();
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        if workbook.sheets.sheet.len() <= 1 {
            return Err(ApiError::new(
                ApiErrorCode::CannotDeleteLastSheet,
                "cannot delete the last worksheet",
            ));
        }
        let Some(index) = workbook
            .sheets
            .sheet
            .iter()
            .position(|sheet| sheet.name.as_str() == name)
        else {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {name}"),
            )
            .with_sheet(name));
        };
        let relationship_id = workbook.sheets.sheet.remove(index).id;
        let _ = wb_part
            .delete_part_by_id(&mut self.doc, relationship_id.as_str())
            .map_err(sdk_err_to_api)?;
        self.invalidate_engine();
        self.normalize_active_sheet_after_delete(index as u32)?;
        Ok(())
    }

    pub fn move_sheet(&mut self, name: impl AsRef<str>, to_index: usize) -> Result<SheetInfo> {
        let name = name.as_ref();
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let len = workbook.sheets.sheet.len();
        let Some(from) = workbook
            .sheets
            .sheet
            .iter()
            .position(|sheet| sheet.name.as_str() == name)
        else {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {name}"),
            )
            .with_sheet(name));
        };
        let to = to_index.min(len.saturating_sub(1));
        if from != to {
            let sheet = workbook.sheets.sheet.remove(from);
            workbook.sheets.sheet.insert(to, sheet);
            self.invalidate_engine();
            self.normalize_active_sheet_after_move(from as u32, to as u32)?;
        }
        self.sheet_info_by_name(name)
    }

    pub fn set_sheet_visibility(
        &mut self,
        name: impl AsRef<str>,
        visibility: SheetVisibility,
    ) -> Result<SheetInfo> {
        let name = name.as_ref();
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let visible_count = workbook
            .sheets
            .sheet
            .iter()
            .filter(|sheet| matches!(sheet.state, None | Some(x::SheetStateValues::Visible)))
            .count();
        let Some(sheet) = workbook
            .sheets
            .sheet
            .iter_mut()
            .find(|sheet| sheet.name.as_str() == name)
        else {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {name}"),
            )
            .with_sheet(name));
        };
        let was_visible = matches!(sheet.state, None | Some(x::SheetStateValues::Visible));
        let becoming_hidden = !matches!(visibility, SheetVisibility::Visible);
        if was_visible && becoming_hidden && visible_count <= 1 {
            return Err(ApiError::new(
                ApiErrorCode::Other,
                "cannot hide the last visible worksheet",
            )
            .with_sheet(name));
        }
        sheet.state = match visibility {
            SheetVisibility::Visible => None,
            SheetVisibility::Hidden => Some(x::SheetStateValues::Hidden),
            SheetVisibility::VeryHidden => Some(x::SheetStateValues::VeryHidden),
        };
        self.ensure_active_sheet_visible()?;
        self.sheet_info_by_name(name)
    }

    pub fn set_active_sheet(&mut self, name: impl AsRef<str>) -> Result<SheetInfo> {
        let name = name.as_ref();
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(index) = workbook
            .sheets
            .sheet
            .iter()
            .position(|sheet| sheet.name.as_str() == name)
        else {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {name}"),
            )
            .with_sheet(name));
        };
        let sheet = &workbook.sheets.sheet[index];
        if !matches!(sheet.state, None | Some(x::SheetStateValues::Visible)) {
            return Err(ApiError::new(
                ApiErrorCode::Other,
                format!("cannot activate hidden sheet: {name}"),
            )
            .with_sheet(name));
        }
        let book_views = workbook.book_views.get_or_insert_with(Default::default);
        if book_views.workbook_view.is_empty() {
            book_views.workbook_view.push(x::WorkbookView::default());
        }
        book_views.workbook_view[0].active_tab = Some(index as u32);
        self.sheet_info_by_name(name)
    }

    fn sheet_info_by_name(&mut self, name: &str) -> Result<SheetInfo> {
        self.sheets()?
            .into_iter()
            .find(|sheet| sheet.name == name)
            .ok_or_else(|| {
                ApiError::new(
                    ApiErrorCode::MissingSheet,
                    format!("sheet not found: {name}"),
                )
                .with_sheet(name)
            })
    }

    fn normalize_active_sheet_after_move(&mut self, from: u32, to: u32) -> Result<()> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(book_views) = workbook.book_views.as_mut() else {
            return Ok(());
        };
        let Some(view) = book_views.workbook_view.first_mut() else {
            return Ok(());
        };
        let Some(active) = view.active_tab else {
            return Ok(());
        };
        let new_active = if active == from {
            to
        } else if from < to && active > from && active <= to {
            active - 1
        } else if to < from && active >= to && active < from {
            active + 1
        } else {
            active
        };
        view.active_tab = Some(new_active);
        Ok(())
    }

    fn ensure_active_sheet_visible(&mut self) -> Result<()> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let states: Vec<bool> = workbook
            .sheets
            .sheet
            .iter()
            .map(|sheet| matches!(sheet.state, None | Some(x::SheetStateValues::Visible)))
            .collect();
        let first_visible = states.iter().position(|visible| *visible);
        let Some(book_views) = workbook.book_views.as_mut() else {
            return Ok(());
        };
        let Some(view) = book_views.workbook_view.first_mut() else {
            return Ok(());
        };
        let active = view.active_tab.unwrap_or(0) as usize;
        let active_hidden = states.get(active).map(|v| !*v).unwrap_or(false);
        if active_hidden {
            if let Some(idx) = first_visible {
                view.active_tab = Some(idx as u32);
            }
        }
        Ok(())
    }

    pub(crate) fn workbook_sheets(&mut self) -> Result<Vec<x::Sheet>> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?;
        Ok(wb_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?
            .sheets
            .sheet
            .clone())
    }

    pub(crate) fn active_sheet_index(&mut self) -> Result<Option<u32>> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?;
        Ok(wb_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?
            .book_views
            .as_ref()
            .and_then(|views| views.workbook_view.first())
            .and_then(|view| view.active_tab))
    }

    fn normalize_active_sheet_after_delete(&mut self, deleted_index: u32) -> Result<()> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let workbook = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(book_views) = workbook.book_views.as_mut() else {
            return Ok(());
        };
        let Some(view) = book_views.workbook_view.first_mut() else {
            return Ok(());
        };
        if let Some(active) = view.active_tab {
            view.active_tab = if active == deleted_index {
                Some(0)
            } else if active > deleted_index {
                Some(active - 1)
            } else {
                Some(active)
            };
        }
        Ok(())
    }

    pub(crate) fn worksheet_parts_by_relationship_id(
        &self,
    ) -> Result<HashMap<String, WorksheetPart>> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?;
        Ok(wb_part
            .worksheet_parts(&self.doc)
            .filter_map(|part| {
                part.relationship_id()
                    .map(|id| (id.to_string(), part.clone()))
            })
            .collect())
    }

    pub(crate) fn worksheet_part_for_sheet(&mut self, sheet_name: &str) -> Result<WorksheetPart> {
        let workbook_sheets = self.workbook_sheets()?;
        let Some(sheet) = workbook_sheets
            .iter()
            .find(|sheet| sheet.name.as_str() == sheet_name)
        else {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {sheet_name}"),
            )
            .with_sheet(sheet_name));
        };
        let relationship_id = sheet.id.as_str().to_string();
        let ws_parts = self.worksheet_parts_by_relationship_id()?;
        ws_parts.get(&relationship_id).cloned().ok_or_else(|| {
            ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("worksheet part not found for sheet: {sheet_name}"),
            )
            .with_sheet(sheet_name)
        })
    }

    pub(crate) fn sheet_exists(&mut self, name: &str) -> Result<bool> {
        Ok(self
            .workbook_sheets()?
            .iter()
            .any(|sheet| sheet.name.as_str() == name))
    }

    pub(crate) fn default_sheet_name(&mut self) -> Result<String> {
        self.workbook_sheets()?
            .first()
            .map(|sheet| sheet.name.as_str().to_string())
            .ok_or_else(|| ApiError::new(ApiErrorCode::MissingSheet, "workbook has no worksheets"))
    }
}
