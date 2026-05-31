use xlcore_io::spreadsheetml as x;
use xlcore_types::{ApiError, ApiErrorCode, FreezeInfo};

use crate::errors::sdk_err_to_api;
use crate::{Result, Workbook};

const MAX_ROW: u32 = 1_048_576;
const MAX_COLUMN: u32 = 16_384;

impl Workbook {
    pub fn set_row_height(
        &mut self,
        sheet: impl AsRef<str>,
        row: u32,
        height: f64,
    ) -> Result<()> {
        let sheet = sheet.as_ref().to_string();
        validate_row(row, &sheet)?;
        validate_size(height, "row height")?;
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let row_entry = ensure_row(ws, row);
        row_entry.height = Some(height);
        row_entry.custom_height = Some(true);
        Ok(())
    }

    pub fn set_row_visible(
        &mut self,
        sheet: impl AsRef<str>,
        row: u32,
        visible: bool,
    ) -> Result<()> {
        let sheet = sheet.as_ref().to_string();
        validate_row(row, &sheet)?;
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let row_entry = ensure_row(ws, row);
        row_entry.hidden = if visible { None } else { Some(true) };
        Ok(())
    }

    pub fn set_column_width(
        &mut self,
        sheet: impl AsRef<str>,
        column: u32,
        width: f64,
    ) -> Result<()> {
        let sheet = sheet.as_ref().to_string();
        validate_column(column, &sheet)?;
        validate_size(width, "column width")?;
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let col = ensure_single_column(ws, column);
        col.width = Some(width);
        col.custom_width = Some(true);
        Ok(())
    }

    pub fn set_column_visible(
        &mut self,
        sheet: impl AsRef<str>,
        column: u32,
        visible: bool,
    ) -> Result<()> {
        let sheet = sheet.as_ref().to_string();
        validate_column(column, &sheet)?;
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let col = ensure_single_column(ws, column);
        col.hidden = if visible { None } else { Some(true) };
        Ok(())
    }

    pub fn set_freeze(
        &mut self,
        sheet: impl AsRef<str>,
        frozen_rows: u32,
        frozen_columns: u32,
    ) -> Result<FreezeInfo> {
        let sheet = sheet.as_ref().to_string();
        if frozen_rows > MAX_ROW || frozen_columns > MAX_COLUMN {
            return Err(ApiError::new(
                ApiErrorCode::InvalidRef,
                "freeze counts exceed sheet bounds",
            )
            .with_sheet(&sheet));
        }
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let views = ws
            .sheet_views
            .get_or_insert_with(|| Box::new(x::SheetViews::default()));
        if views.x_sheet_view.is_empty() {
            views.x_sheet_view.push(x::SheetView::default());
        }
        let view = &mut views.x_sheet_view[0];
        if frozen_rows == 0 && frozen_columns == 0 {
            view.pane = None;
        } else {
            let top_left = format!(
                "{}{}",
                xlcore_io::col_label(frozen_columns + 1),
                frozen_rows + 1
            );
            let active_pane = if frozen_rows > 0 && frozen_columns > 0 {
                x::PaneValues::BottomRight
            } else if frozen_rows > 0 {
                x::PaneValues::BottomLeft
            } else {
                x::PaneValues::TopRight
            };
            view.pane = Some(x::Pane {
                horizontal_split: if frozen_columns > 0 {
                    Some(frozen_columns as f64)
                } else {
                    None
                },
                vertical_split: if frozen_rows > 0 {
                    Some(frozen_rows as f64)
                } else {
                    None
                },
                top_left_cell: Some(top_left),
                active_pane: Some(active_pane),
                state: Some(x::PaneStateValues::Frozen),
            });
        }
        Ok(FreezeInfo {
            sheet,
            frozen_rows,
            frozen_columns,
        })
    }

    pub fn get_freeze(&mut self, sheet: impl AsRef<str>) -> Result<FreezeInfo> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let (frozen_rows, frozen_columns) = ws
            .sheet_views
            .as_ref()
            .and_then(|views| views.x_sheet_view.first())
            .and_then(|view| view.pane.as_ref())
            .filter(|pane| matches!(pane.state, Some(x::PaneStateValues::Frozen)))
            .map(|pane| {
                (
                    pane.vertical_split.unwrap_or(0.0) as u32,
                    pane.horizontal_split.unwrap_or(0.0) as u32,
                )
            })
            .unwrap_or((0, 0));
        Ok(FreezeInfo {
            sheet,
            frozen_rows,
            frozen_columns,
        })
    }
}

fn validate_row(row: u32, sheet: &str) -> Result<()> {
    if row == 0 || row > MAX_ROW {
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("row index out of bounds: {row}"),
        )
        .with_sheet(sheet));
    }
    Ok(())
}

fn validate_column(column: u32, sheet: &str) -> Result<()> {
    if column == 0 || column > MAX_COLUMN {
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("column index out of bounds: {column}"),
        )
        .with_sheet(sheet));
    }
    Ok(())
}

fn validate_size(value: f64, label: &str) -> Result<()> {
    if !value.is_finite() || value < 0.0 {
        return Err(ApiError::new(
            ApiErrorCode::InvalidRef,
            format!("{label} must be a non-negative finite number"),
        ));
    }
    Ok(())
}

fn ensure_row(ws: &mut x::Worksheet, row: u32) -> &mut x::Row {
    let pos = match ws
        .x_sheet_data
        .x_row
        .binary_search_by_key(&row, |existing| existing.row_index.unwrap_or(u32::MAX))
    {
        Ok(pos) => pos,
        Err(pos) => {
            ws.x_sheet_data.x_row.insert(
                pos,
                x::Row {
                    row_index: Some(row),
                    ..Default::default()
                },
            );
            pos
        }
    };
    &mut ws.x_sheet_data.x_row[pos]
}

fn ensure_single_column(ws: &mut x::Worksheet, column: u32) -> &mut x::Column {
    if ws.x_cols.is_empty() {
        ws.x_cols.push(x::Columns::default());
    }
    let cols = &mut ws.x_cols[0];
    if let Some(pos) = cols
        .x_col
        .iter()
        .position(|c| c.min == column && c.max == column)
    {
        return &mut cols.x_col[pos];
    }
    if let Some(pos) = cols
        .x_col
        .iter()
        .position(|c| c.min <= column && column <= c.max)
    {
        let entry = cols.x_col.remove(pos);
        let mut insert_at = pos;
        if entry.min < column {
            let mut left = entry.clone();
            left.max = column - 1;
            cols.x_col.insert(insert_at, left);
            insert_at += 1;
        }
        let mut mid = entry.clone();
        mid.min = column;
        mid.max = column;
        cols.x_col.insert(insert_at, mid);
        let mid_index = insert_at;
        insert_at += 1;
        if entry.max > column {
            let mut right = entry.clone();
            right.min = column + 1;
            cols.x_col.insert(insert_at, right);
        }
        return &mut cols.x_col[mid_index];
    }
    let pos = cols
        .x_col
        .iter()
        .position(|c| c.min > column)
        .unwrap_or(cols.x_col.len());
    cols.x_col.insert(
        pos,
        x::Column {
            min: column,
            max: column,
            ..Default::default()
        },
    );
    &mut cols.x_col[pos]
}
