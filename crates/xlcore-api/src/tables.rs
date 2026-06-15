use ooxmlsdk::parts::table_definition_part::TableDefinitionPart;
use ooxmlsdk::sdk::SdkPart;
use ooxmlsdk::simple_type::BooleanValue;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiError, ApiErrorCode, TableColumnInfo, TableColumnPatch, TableInfo, TablePatch,
    TableStylePatch, TableStyleSettings, TableTotalsFunction,
};

use crate::errors::sdk_err_to_api;
use crate::refs::{parse_range_a1, qualify_ref, ranges_overlap};
use crate::xml::{ensure_cell, mark_formulas_stale};
use crate::{Result, Workbook};

impl Workbook {
    pub fn tables(&mut self, sheet: Option<&str>) -> Result<Vec<TableInfo>> {
        let sheet_names: Vec<String> = match sheet {
            Some(name) => {
                if !self.sheet_exists(name)? {
                    return Err(ApiError::new(
                        ApiErrorCode::MissingSheet,
                        format!("sheet not found: {name}"),
                    )
                    .with_sheet(name));
                }
                vec![name.to_string()]
            }
            None => self
                .workbook_sheets()?
                .iter()
                .map(|s| s.name.as_str().to_string())
                .collect(),
        };
        let mut out = Vec::new();
        for sheet_name in &sheet_names {
            let ws_part = self.worksheet_part_for_sheet(sheet_name)?;
            let table_parts: Vec<_> = ws_part.table_definition_parts(&self.doc).collect();
            for tp in &table_parts {
                let table = tp.root_element(&mut self.doc).map_err(sdk_err_to_api)?;
                if let Some(info) = info_from_table(sheet_name, table) {
                    out.push(info);
                }
            }
        }
        Ok(out)
    }

    pub fn set_table(
        &mut self,
        sheet: impl AsRef<str>,
        mut patch: TablePatch,
    ) -> Result<TableInfo> {
        let sheet = sheet.as_ref();
        if !sheet.is_empty() {
            if let Some(reference) = patch.reference.as_deref() {
                patch.reference = Some(qualify_ref(sheet, reference)?);
            }
        }
        validate_table_name(&patch.name)?;
        if let Some(dn) = patch.display_name.as_deref() {
            validate_table_name(dn)?;
        }

        let existing = self.locate_table(&patch.name)?;

        if let Some((sheet, rid)) = existing {
            self.update_table(&sheet, &rid, &patch)
        } else {
            self.create_table(&patch)
        }
    }

    pub fn remove_table(&mut self, name: impl AsRef<str>) -> Result<Option<TableInfo>> {
        let name = name.as_ref();
        let Some((sheet, rid)) = self.locate_table(name)? else {
            return Ok(None);
        };
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let tp = ws_part
            .table_definition_parts(&self.doc)
            .find(|p| p.relationship_id() == Some(rid.as_str()))
            .ok_or_else(|| {
                ApiError::new(ApiErrorCode::Other, "table part disappeared").with_sheet(&sheet)
            })?
            .clone();
        let table = tp.root_element(&mut self.doc).map_err(sdk_err_to_api)?;
        let info = info_from_table(&sheet, table);
        let _ = ws_part
            .delete_part_by_id(&mut self.doc, rid.as_str())
            .map_err(sdk_err_to_api)?;
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        if let Some(tp) = ws.table_parts.as_mut() {
            tp.table_part.retain(|p| p.id.as_str() != rid);
            tp.count = Some(tp.table_part.len() as u32);
            if tp.table_part.is_empty() {
                ws.table_parts = None;
            }
        }
        Ok(info)
    }

    fn locate_table(&mut self, name: &str) -> Result<Option<(String, String)>> {
        let sheet_names: Vec<String> = self
            .workbook_sheets()?
            .iter()
            .map(|s| s.name.as_str().to_string())
            .collect();
        for sheet in &sheet_names {
            let ws_part = self.worksheet_part_for_sheet(sheet)?;
            let table_parts: Vec<_> = ws_part.table_definition_parts(&self.doc).collect();
            for tp in &table_parts {
                let rid = match tp.relationship_id() {
                    Some(r) => r.to_string(),
                    None => continue,
                };
                let table = tp.root_element(&mut self.doc).map_err(sdk_err_to_api)?;
                if table_matches_name(table, name) {
                    return Ok(Some((sheet.clone(), rid)));
                }
            }
        }
        Ok(None)
    }

    fn next_table_id(&mut self) -> Result<u32> {
        let sheet_names: Vec<String> = self
            .workbook_sheets()?
            .iter()
            .map(|s| s.name.as_str().to_string())
            .collect();
        let mut max = 0u32;
        for sheet in &sheet_names {
            let ws_part = self.worksheet_part_for_sheet(sheet)?;
            let table_parts: Vec<_> = ws_part.table_definition_parts(&self.doc).collect();
            for tp in &table_parts {
                let table = tp.root_element(&mut self.doc).map_err(sdk_err_to_api)?;
                let id: u32 = table.id.into();
                if id > max {
                    max = id;
                }
            }
        }
        Ok(max + 1)
    }

    fn create_table(&mut self, patch: &TablePatch) -> Result<TableInfo> {
        let reference = patch.reference.as_deref().ok_or_else(|| {
            ApiError::new(
                ApiErrorCode::InvalidTable,
                "reference is required when creating a table",
            )
        })?;
        let range_ref = self.resolve_range_ref(reference)?;
        self.ensure_no_table_overlap(&range_ref.sheet, None, &range_ref)?;

        let display_name = patch
            .display_name
            .clone()
            .unwrap_or_else(|| patch.name.clone());
        let header_rows = patch.header_row_count.unwrap_or(1);
        let totals_rows = patch.totals_row_count.unwrap_or(0);
        let has_filter = patch.has_auto_filter.unwrap_or(header_rows > 0);

        let column_count = range_ref.end_column - range_ref.start_column + 1;
        validate_table_geometry(header_rows, totals_rows, &range_ref)?;
        let total_rows = range_ref.end_row - range_ref.start_row + 1;
        if header_rows + totals_rows > total_rows {
            return Err(ApiError::new(
                ApiErrorCode::InvalidTable,
                "header + totals rows exceed table range",
            )
            .with_sheet(&range_ref.sheet)
            .with_ref(reference));
        }

        let header_names = if header_rows > 0 {
            read_header_names(self, &range_ref)?
        } else {
            Vec::new()
        };

        let column_patches: Vec<TableColumnPatch> = patch.columns.clone().unwrap_or_default();
        if !column_patches.is_empty() && column_patches.len() as u32 != column_count {
            return Err(ApiError::new(
                ApiErrorCode::InvalidTable,
                format!(
                    "columns length {} does not match table column count {}",
                    column_patches.len(),
                    column_count
                ),
            )
            .with_sheet(&range_ref.sheet)
            .with_ref(reference));
        }

        let resolved_names =
            resolve_unique_column_names(column_count, &header_names, &column_patches)?;

        if header_rows > 0 {
            write_header_row(self, &range_ref, &resolved_names)?;
        }

        let id = self.next_table_id()?;
        let new_ref = range_ref.range_reference();
        let auto_filter = if has_filter {
            Some(Box::new(x::AutoFilter {
                reference: Some(new_ref.clone().into()),
                ..Default::default()
            }))
        } else {
            None
        };

        let columns: Vec<x::TableColumn> = (0..column_count)
            .map(|i| {
                build_table_column(
                    i,
                    &resolved_names[i as usize],
                    column_patches.get(i as usize),
                )
            })
            .collect();
        let mut table = x::Table {
            xmlns: crate::ooxml_header::spreadsheetml_default_only(),
            xml_header: crate::ooxml_header::STANDALONE,
            id: id.into(),
            name: Some(patch.name.clone().into()),
            display_name: display_name.clone().into(),
            reference: new_ref.clone().into(),
            header_row_count: Some(header_rows.into()),
            totals_row_count: if totals_rows > 0 {
                Some(totals_rows.into())
            } else {
                None
            },
            totals_row_shown: Some((totals_rows > 0).into()),
            auto_filter,
            table_columns: Box::new(x::TableColumns {
                count: Some(column_count.into()),
                table_column: columns,
            }),
            table_style_info: patch.style.as_ref().map(build_table_style_info),
            ..Default::default()
        };
        if header_rows == 0 {
            table.header_row_count = Some(0u32.into());
        }

        let ws_part = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let table_part: TableDefinitionPart = ws_part
            .add_new_part_auto_id(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let rid = table_part
            .relationship_id()
            .ok_or_else(|| {
                ApiError::new(ApiErrorCode::Other, "table part missing relationship id")
            })?
            .to_string();
        table_part
            .set_root_element(&mut self.doc, table)
            .map_err(sdk_err_to_api)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let tp = ws.table_parts.get_or_insert_with(x::TableParts::default);
        tp.table_part.push(x::TablePart {
            id: rid.clone().into(),
            ..Default::default()
        });
        tp.count = Some(tp.table_part.len() as u32);

        mark_formulas_stale(&mut self.doc)?;

        let table_ref = self.worksheet_part_for_sheet(&range_ref.sheet)?;
        let tp = table_ref
            .table_definition_parts(&self.doc)
            .find(|p| p.relationship_id() == Some(rid.as_str()))
            .ok_or_else(|| ApiError::new(ApiErrorCode::Other, "added table part not found"))?;
        let table = tp.root_element(&mut self.doc).map_err(sdk_err_to_api)?;
        Ok(
            info_from_table(&range_ref.sheet, table).unwrap_or_else(|| TableInfo {
                name: patch.name.clone(),
                display_name,
                sheet: range_ref.sheet.clone(),
                reference: new_ref,
                start_row: range_ref.start_row,
                start_column: range_ref.start_column,
                end_row: range_ref.end_row,
                end_column: range_ref.end_column,
                header_row_count: header_rows,
                totals_row_count: totals_rows,
                has_auto_filter: has_filter,
                columns: Vec::new(),
                style: None,
            }),
        )
    }

    fn update_table(&mut self, sheet: &str, rid: &str, patch: &TablePatch) -> Result<TableInfo> {
        let ws_part = self.worksheet_part_for_sheet(sheet)?;
        let tp = ws_part
            .table_definition_parts(&self.doc)
            .find(|p| p.relationship_id() == Some(rid))
            .ok_or_else(|| {
                ApiError::new(ApiErrorCode::Other, "table part disappeared").with_sheet(sheet)
            })?
            .clone();

        let new_range = if let Some(reference) = patch.reference.as_deref() {
            let r = self.resolve_range_ref(reference)?;
            if r.sheet != sheet {
                return Err(ApiError::new(
                    ApiErrorCode::InvalidTable,
                    format!(
                        "cannot move table across sheets (from {sheet} to {})",
                        r.sheet
                    ),
                )
                .with_sheet(sheet)
                .with_ref(reference));
            }
            self.ensure_no_table_overlap(sheet, Some(rid), &r)?;
            Some(r)
        } else {
            None
        };

        let display_name_clone = patch.display_name.clone();
        let style_patch = patch.style.clone();
        let columns_patch = patch.columns.clone();
        let header_row_count_opt = patch.header_row_count;
        let totals_row_count_opt = patch.totals_row_count;
        let has_auto_filter_opt = patch.has_auto_filter;
        let new_name = patch.name.clone();

        let new_range_ref_string = new_range.as_ref().map(|r| r.range_reference());

        let header_resolved_names: Option<Vec<String>> = if let Some(cols) = columns_patch.as_ref()
        {
            let table = tp.root_element(&mut self.doc).map_err(sdk_err_to_api)?;
            let column_count = match new_range.as_ref() {
                Some(r) => r.end_column - r.start_column + 1,
                None => table.table_columns.table_column.len() as u32,
            };
            if cols.len() as u32 != column_count {
                return Err(ApiError::new(
                    ApiErrorCode::InvalidTable,
                    format!(
                        "columns length {} does not match table column count {}",
                        cols.len(),
                        column_count
                    ),
                )
                .with_sheet(sheet));
            }
            let existing: Vec<String> = table
                .table_columns
                .table_column
                .iter()
                .map(|c| c.name.as_str().to_string())
                .collect();
            let resolved = resolve_unique_column_names(column_count, &existing, cols)?;
            Some(resolved)
        } else {
            None
        };

        let table = tp.root_element_mut(&mut self.doc).map_err(sdk_err_to_api)?;

        if let Some(new_ref) = new_range_ref_string.as_ref() {
            table.reference = new_ref.clone().into();
            if let Some(af) = table.auto_filter.as_mut() {
                af.reference = Some(new_ref.clone().into());
            }
        }
        table.name = Some(new_name.into());
        if let Some(dn) = display_name_clone {
            table.display_name = dn.into();
        }
        if let Some(h) = header_row_count_opt {
            table.header_row_count = Some(h.into());
        }
        if let Some(t) = totals_row_count_opt {
            if t > 0 {
                table.totals_row_count = Some(t.into());
                table.totals_row_shown = Some(true.into());
            } else {
                table.totals_row_count = None;
                table.totals_row_shown = Some(false.into());
            }
        }
        if let Some(filter) = has_auto_filter_opt {
            if filter {
                let new_ref = table.reference.as_str().to_string();
                table
                    .auto_filter
                    .get_or_insert_with(|| Box::new(x::AutoFilter::default()))
                    .reference = Some(new_ref.into());
            } else {
                table.auto_filter = None;
            }
        }
        if let Some(style) = style_patch.as_ref() {
            let info = table
                .table_style_info
                .get_or_insert_with(x::TableStyleInfo::default);
            if let Some(name) = style.name.as_ref() {
                info.name = Some(name.clone().into());
            }
            if let Some(v) = style.show_first_column {
                info.show_first_column = Some(v.into());
            }
            if let Some(v) = style.show_last_column {
                info.show_last_column = Some(v.into());
            }
            if let Some(v) = style.show_row_stripes {
                info.show_row_stripes = Some(v.into());
            }
            if let Some(v) = style.show_column_stripes {
                info.show_column_stripes = Some(v.into());
            }
        }

        if let Some(cols) = columns_patch.as_ref() {
            let names = header_resolved_names.unwrap();
            let column_count = names.len() as u32;
            let mut new_columns: Vec<x::TableColumn> = Vec::with_capacity(column_count as usize);
            for i in 0..column_count {
                let prev = table.table_columns.table_column.get(i as usize).cloned();
                let mut col = match prev {
                    Some(mut c) => {
                        c.id = ((i + 1) as u32).into();
                        c
                    }
                    None => x::TableColumn {
                        id: ((i + 1) as u32).into(),
                        name: names[i as usize].clone().into(),
                        ..Default::default()
                    },
                };
                col.name = names[i as usize].clone().into();
                apply_column_patch(&mut col, &cols[i as usize]);
                new_columns.push(col);
            }
            table.table_columns.table_column = new_columns;
            table.table_columns.count = Some(column_count.into());
        } else if let Some(r) = new_range.as_ref() {
            let column_count = r.end_column - r.start_column + 1;
            let existing_len = table.table_columns.table_column.len() as u32;
            if column_count > existing_len {
                for i in existing_len..column_count {
                    table.table_columns.table_column.push(x::TableColumn {
                        id: ((i + 1) as u32).into(),
                        name: format!("Column{}", i + 1).into(),
                        ..Default::default()
                    });
                }
            } else if column_count < existing_len {
                table
                    .table_columns
                    .table_column
                    .truncate(column_count as usize);
            }
            table.table_columns.count = Some(column_count.into());
        }

        let final_header_count: u32 = table.header_row_count.map(|v| v.into()).unwrap_or(1);
        let column_names: Vec<String> = table
            .table_columns
            .table_column
            .iter()
            .map(|c| c.name.as_str().to_string())
            .collect();
        let header_range = new_range.as_ref().cloned();

        if final_header_count > 0 {
            let target_range = match header_range {
                Some(r) => r,
                None => {
                    let cur_ref = table.reference.as_str().to_string();
                    self.resolve_range_ref(&format!(
                        "{}!{}",
                        crate::refs::quote_sheet_name(sheet),
                        cur_ref
                    ))?
                }
            };
            write_header_row(self, &target_range, &column_names)?;
        }

        mark_formulas_stale(&mut self.doc)?;

        let ws_part = self.worksheet_part_for_sheet(sheet)?;
        let tp = ws_part
            .table_definition_parts(&self.doc)
            .find(|p| p.relationship_id() == Some(rid))
            .ok_or_else(|| ApiError::new(ApiErrorCode::Other, "table part disappeared"))?;
        let table = tp.root_element(&mut self.doc).map_err(sdk_err_to_api)?;
        info_from_table(sheet, table)
            .ok_or_else(|| ApiError::new(ApiErrorCode::Other, "failed to read back table"))
    }

    fn ensure_no_table_overlap(
        &mut self,
        sheet: &str,
        skip_rid: Option<&str>,
        range_ref: &crate::refs::ResolvedRangeRef,
    ) -> Result<()> {
        let ws_part = self.worksheet_part_for_sheet(sheet)?;
        let table_parts: Vec<_> = ws_part.table_definition_parts(&self.doc).collect();
        for tp in &table_parts {
            if let Some(rid) = tp.relationship_id() {
                if skip_rid == Some(rid) {
                    continue;
                }
            }
            let table = tp.root_element(&mut self.doc).map_err(sdk_err_to_api)?;
            let Some((r1, c1, r2, c2)) = parse_range_a1(table.reference.as_str()) else {
                continue;
            };
            if ranges_overlap(
                range_ref.start_row,
                range_ref.start_column,
                range_ref.end_row,
                range_ref.end_column,
                r1,
                c1,
                r2,
                c2,
            ) {
                return Err(ApiError::new(
                    ApiErrorCode::InvalidTable,
                    format!(
                        "table range {} overlaps existing table {}",
                        range_ref.range_reference(),
                        table.reference.as_str()
                    ),
                )
                .with_sheet(sheet));
            }
        }
        Ok(())
    }
}

fn table_matches_name(table: &x::Table, name: &str) -> bool {
    if let Some(n) = table.name.as_ref() {
        if n.as_str() == name {
            return true;
        }
    }
    table.display_name.as_str() == name
}

fn info_from_table(sheet: &str, table: &x::Table) -> Option<TableInfo> {
    let (r1, c1, r2, c2) = parse_range_a1(table.reference.as_str())?;
    let name = table
        .name
        .as_ref()
        .map(|n| n.as_str().to_string())
        .unwrap_or_else(|| table.display_name.as_str().to_string());
    let columns: Vec<TableColumnInfo> = table
        .table_columns
        .table_column
        .iter()
        .map(|c| TableColumnInfo {
            id: c.id.into(),
            name: c.name.as_str().to_string(),
            totals_function: c
                .totals_row_function
                .as_ref()
                .map(totals_from_ooxml)
                .unwrap_or(TableTotalsFunction::None),
            totals_label: c.totals_row_label.as_ref().map(|s| s.as_str().to_string()),
            totals_formula: c
                .totals_row_formula
                .as_ref()
                .and_then(|f| f.xml_content.as_deref().map(str::to_string)),
            calculated_column_formula: c
                .calculated_column_formula
                .as_ref()
                .and_then(|f| f.xml_content.as_deref().map(str::to_string)),
        })
        .collect();
    let style = table.table_style_info.as_ref().map(|s| TableStyleSettings {
        name: s.name.as_ref().map(|n| n.as_str().to_string()),
        show_first_column: bool::from(
            s.show_first_column
                .unwrap_or(BooleanValue::from_bool(false)),
        ),
        show_last_column: bool::from(s.show_last_column.unwrap_or(BooleanValue::from_bool(false))),
        show_row_stripes: bool::from(s.show_row_stripes.unwrap_or(BooleanValue::from_bool(false))),
        show_column_stripes: bool::from(
            s.show_column_stripes
                .unwrap_or(BooleanValue::from_bool(false)),
        ),
    });
    let header_row_count = table.header_row_count.map(u32::from).unwrap_or(1);
    let totals_row_count = table.totals_row_count.map(u32::from).unwrap_or(0);
    Some(TableInfo {
        name,
        display_name: table.display_name.as_str().to_string(),
        sheet: sheet.to_string(),
        reference: format!(
            "{}{}:{}{}",
            xlcore_io::col_label(c1),
            r1,
            xlcore_io::col_label(c2),
            r2,
        ),
        start_row: r1,
        start_column: c1,
        end_row: r2,
        end_column: c2,
        header_row_count,
        totals_row_count,
        has_auto_filter: table.auto_filter.is_some(),
        columns,
        style,
    })
}

fn totals_to_ooxml(value: TableTotalsFunction) -> Option<x::TotalsRowFunctionValues> {
    match value {
        TableTotalsFunction::None => None,
        TableTotalsFunction::Sum => Some(x::TotalsRowFunctionValues::Sum),
        TableTotalsFunction::Average => Some(x::TotalsRowFunctionValues::Average),
        TableTotalsFunction::Count => Some(x::TotalsRowFunctionValues::Count),
        TableTotalsFunction::CountNums => Some(x::TotalsRowFunctionValues::CountNumbers),
        TableTotalsFunction::Min => Some(x::TotalsRowFunctionValues::Minimum),
        TableTotalsFunction::Max => Some(x::TotalsRowFunctionValues::Maximum),
        TableTotalsFunction::StdDev => Some(x::TotalsRowFunctionValues::StandardDeviation),
        TableTotalsFunction::Var => Some(x::TotalsRowFunctionValues::Variance),
        TableTotalsFunction::Custom => Some(x::TotalsRowFunctionValues::Custom),
    }
}

fn totals_from_ooxml(value: &x::TotalsRowFunctionValues) -> TableTotalsFunction {
    match value {
        x::TotalsRowFunctionValues::None => TableTotalsFunction::None,
        x::TotalsRowFunctionValues::Sum => TableTotalsFunction::Sum,
        x::TotalsRowFunctionValues::Average => TableTotalsFunction::Average,
        x::TotalsRowFunctionValues::Count => TableTotalsFunction::Count,
        x::TotalsRowFunctionValues::CountNumbers => TableTotalsFunction::CountNums,
        x::TotalsRowFunctionValues::Minimum => TableTotalsFunction::Min,
        x::TotalsRowFunctionValues::Maximum => TableTotalsFunction::Max,
        x::TotalsRowFunctionValues::StandardDeviation => TableTotalsFunction::StdDev,
        x::TotalsRowFunctionValues::Variance => TableTotalsFunction::Var,
        x::TotalsRowFunctionValues::Custom => TableTotalsFunction::Custom,
    }
}

fn build_table_column(index: u32, name: &str, patch: Option<&TableColumnPatch>) -> x::TableColumn {
    let mut col = x::TableColumn {
        id: ((index + 1) as u32).into(),
        name: name.to_string().into(),
        ..Default::default()
    };
    if let Some(p) = patch {
        apply_column_patch(&mut col, p);
    }
    col
}

fn apply_column_patch(col: &mut x::TableColumn, patch: &TableColumnPatch) {
    if let Some(fun) = patch.totals_function {
        col.totals_row_function = totals_to_ooxml(fun);
    }
    if let Some(label) = patch.totals_label.as_ref() {
        col.totals_row_label = Some(label.clone().into());
    }
    if let Some(formula) = patch.totals_formula.as_ref() {
        col.totals_row_formula = Some(x::TotalsRowFormula(x::TableFormulaType {
            xml_content: Some(formula.clone().into()),
            ..Default::default()
        }));
    }
    if let Some(formula) = patch.calculated_column_formula.as_ref() {
        col.calculated_column_formula = Some(x::CalculatedColumnFormula(x::TableFormulaType {
            xml_content: Some(formula.clone().into()),
            ..Default::default()
        }));
    }
}

fn build_table_style_info(patch: &TableStylePatch) -> x::TableStyleInfo {
    x::TableStyleInfo {
        name: patch.name.clone().map(Into::into),
        show_first_column: patch.show_first_column.map(Into::into),
        show_last_column: patch.show_last_column.map(Into::into),
        show_row_stripes: patch.show_row_stripes.map(Into::into),
        show_column_stripes: patch.show_column_stripes.map(Into::into),
    }
}

fn validate_table_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidTable,
            "table name is empty",
        ));
    }
    if name.len() > 255 {
        return Err(ApiError::new(
            ApiErrorCode::InvalidTable,
            "table name exceeds 255 characters",
        ));
    }
    let first = name.chars().next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '_' || first == '\\') {
        return Err(ApiError::new(
            ApiErrorCode::InvalidTable,
            "table name must start with a letter, underscore, or backslash",
        ));
    }
    if name.chars().any(|c| c.is_whitespace()) {
        return Err(ApiError::new(
            ApiErrorCode::InvalidTable,
            "table name cannot contain whitespace",
        ));
    }
    for c in name.chars() {
        let ok = c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '\\';
        if !ok {
            return Err(ApiError::new(
                ApiErrorCode::InvalidTable,
                format!("table name contains invalid character: {c:?}"),
            ));
        }
    }
    Ok(())
}

fn validate_table_geometry(
    header_rows: u32,
    totals_rows: u32,
    range_ref: &crate::refs::ResolvedRangeRef,
) -> Result<()> {
    if header_rows > 1 {
        return Err(
            ApiError::new(ApiErrorCode::InvalidTable, "headerRowCount must be 0 or 1")
                .with_sheet(&range_ref.sheet),
        );
    }
    if totals_rows > 1 {
        return Err(
            ApiError::new(ApiErrorCode::InvalidTable, "totalsRowCount must be 0 or 1")
                .with_sheet(&range_ref.sheet),
        );
    }
    Ok(())
}

fn resolve_unique_column_names(
    column_count: u32,
    header_or_existing: &[String],
    patches: &[TableColumnPatch],
) -> Result<Vec<String>> {
    let mut names: Vec<String> = Vec::with_capacity(column_count as usize);
    let mut seen = std::collections::HashSet::new();
    for i in 0..column_count {
        let from_patch = patches
            .get(i as usize)
            .and_then(|p| p.name.as_ref())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let from_header = header_or_existing
            .get(i as usize)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let base = from_patch
            .or(from_header)
            .unwrap_or_else(|| format!("Column{}", i + 1));
        let mut candidate = base.clone();
        let mut suffix = 2u32;
        while !seen.insert(candidate.clone()) {
            candidate = format!("{base}{suffix}");
            suffix += 1;
        }
        names.push(candidate);
    }
    Ok(names)
}

fn read_header_names(
    workbook: &mut Workbook,
    range_ref: &crate::refs::ResolvedRangeRef,
) -> Result<Vec<String>> {
    let mut out = Vec::with_capacity((range_ref.end_column - range_ref.start_column + 1) as usize);
    let shared_strings = crate::xml::load_shared_strings(&mut workbook.doc);
    let ws_part = workbook.worksheet_part_for_sheet(&range_ref.sheet)?;
    let ws = ws_part
        .root_element(&mut workbook.doc)
        .map_err(sdk_err_to_api)?;
    let header_row_index = range_ref.start_row;
    let row = ws
        .sheet_data
        .row
        .iter()
        .find(|r| r.row_index == Some(header_row_index));
    for col in range_ref.start_column..=range_ref.end_column {
        let cell = row.and_then(|r| {
            r.cell.iter().find(|c| {
                c.cell_reference
                    .as_ref()
                    .and_then(|s| xlcore_io::parse_a1(s.as_str()))
                    .map(|(_, cc)| cc == col)
                    .unwrap_or(false)
            })
        });
        let value = match cell {
            Some(cell) => {
                let raw = cell
                    .cell_value
                    .as_ref()
                    .and_then(|v| v.xml_content.as_deref());
                match crate::xml::read_cell_value(cell, raw, &shared_strings) {
                    xlcore_types::ApiCellValue::String(s) => s,
                    xlcore_types::ApiCellValue::Number(n) => n.to_string(),
                    xlcore_types::ApiCellValue::Boolean(b) => b.to_string(),
                    xlcore_types::ApiCellValue::Error(e) => e,
                    xlcore_types::ApiCellValue::Blank => String::new(),
                }
            }
            None => String::new(),
        };
        out.push(value);
    }
    Ok(out)
}

fn write_header_row(
    workbook: &mut Workbook,
    range_ref: &crate::refs::ResolvedRangeRef,
    names: &[String],
) -> Result<()> {
    let ws_part = workbook.worksheet_part_for_sheet(&range_ref.sheet)?;
    let ws = ws_part
        .root_element_mut(&mut workbook.doc)
        .map_err(sdk_err_to_api)?;
    let header_row_index = range_ref.start_row;
    for (i, col) in (range_ref.start_column..=range_ref.end_column).enumerate() {
        let cell = ensure_cell(ws, header_row_index, col);
        cell.cell_formula = None;
        cell.cell_value = None;
        cell.data_type = Some(x::CellValues::InlineString);
        cell.inline_string = Some(Box::new(x::InlineString {
            text: Some(x::Text(x::XstringType {
                xml_content: Some(names[i].clone()),
                ..Default::default()
            })),
            ..Default::default()
        }));
    }
    Ok(())
}
