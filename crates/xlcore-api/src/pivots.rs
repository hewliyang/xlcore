use std::collections::HashMap;

use ooxmlsdk::parts::pivot_table_cache_definition_part::PivotTableCacheDefinitionPart;
use ooxmlsdk::parts::pivot_table_cache_records_part::PivotTableCacheRecordsPart;
use ooxmlsdk::parts::pivot_table_part::PivotTablePart;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main::{
    DataConsolidateFunctionValues, PivotTableAxisValues,
};
use ooxmlsdk::sdk::SdkPart;
use ooxmlsdk::simple_type::BooleanValue;

use xlcore_types::{
    ApiCellValue as CellValue, ApiError, ApiErrorCode, PivotAggregation, PivotCellRole,
    PivotDataField, PivotFieldFilter, PivotGrid, PivotGridCell, PivotInfo, PivotPatch, PivotUpdate,
};

use crate::errors::sdk_err_to_api;
use crate::refs::{quote_sheet_name, ResolvedCellRef};
use crate::{Result, Workbook};

fn pivot_info_to_patch(info: &PivotInfo) -> PivotPatch {
    PivotPatch {
        sheet: info.sheet.clone(),
        anchor_cell: info.anchor_cell.clone(),
        source_ref: info.source_ref.clone(),
        name: Some(info.name.clone()),
        row_fields: info.row_fields.clone(),
        column_fields: info.column_fields.clone(),
        filter_fields: info.filter_fields.clone(),
        data_fields: info.data_fields.clone(),
        hidden_items: info.hidden_items.clone(),
    }
}

const SPREADSHEETML: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const RELATIONSHIPS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

enum FieldRole {
    Row,
    Column,
    Filter,
    Data,
    Unused,
}

struct SourceColumn {
    name: String,
    #[allow(dead_code)]
    role: FieldRole,
    numeric: bool,
    distinct: Vec<CellValue>,
    distinct_index: HashMap<String, usize>,
}

struct PivotPrep {
    columns: Vec<SourceColumn>,
    headers: Vec<String>,
    anchor: ResolvedCellRef,
    source_sheet: String,
    source_a1: String,
    data_rows: Vec<Vec<CellValue>>,
}

fn grid_from_cells(
    cells: &[xlcore_export::Cell],
    memo: Option<xlcore_export::PivotStyleIndices>,
) -> PivotGrid {
    let Some(st) = memo else {
        return PivotGrid {
            rows: 0,
            cols: 0,
            cells: Vec::new(),
        };
    };
    let min_r = cells.iter().map(|c| c.r).min().unwrap_or(1);
    let min_c = cells.iter().map(|c| c.c).min().unwrap_or(1);
    let mut rows = 0;
    let mut cols = 0;
    let grid_cells = cells
        .iter()
        .map(|c| {
            let row = c.r - min_r;
            let col = c.c - min_c;
            rows = rows.max(row + 1);
            cols = cols.max(col + 1);
            let role = match c.style_index {
                Some(i) if i == st.header() => PivotCellRole::Header,
                Some(i) if i == st.total_label() => PivotCellRole::TotalLabel,
                Some(i) if i == st.total_value() => PivotCellRole::TotalValue,
                _ if c.kind == "n" => PivotCellRole::Value,
                _ => PivotCellRole::Label,
            };
            PivotGridCell {
                row,
                col,
                role,
                kind: c.kind.clone(),
                value: c.value.clone(),
            }
        })
        .collect();
    PivotGrid {
        rows,
        cols,
        cells: grid_cells,
    }
}

impl Workbook {
    pub fn pivots(&mut self, sheet: Option<&str>) -> Result<Vec<PivotInfo>> {
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
            let pivot_parts: Vec<PivotTablePart> = ws_part.pivot_table_parts(&self.doc).collect();
            for pp in &pivot_parts {
                let id = pp.relationship_id().unwrap_or_default().to_string();
                let def = pp
                    .root_element(&mut self.doc)
                    .map_err(sdk_err_to_api)?
                    .clone();
                let cache = pp
                    .pivot_table_cache_definition_part(&self.doc)
                    .and_then(|c| c.root_element(&mut self.doc).ok().cloned());

                let field_names: Vec<String> = cache
                    .as_ref()
                    .map(|c| {
                        c.cache_fields
                            .cache_field
                            .iter()
                            .map(|f| f.name.as_str().to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                let name_of = |idx: i32| -> String {
                    if idx < 0 {
                        "Values".to_string()
                    } else {
                        field_names
                            .get(idx as usize)
                            .cloned()
                            .unwrap_or_else(|| format!("Field{idx}"))
                    }
                };

                let row_fields = def
                    .row_fields
                    .as_ref()
                    .map(|rf| rf.field.iter().map(|f| name_of(f.index)).collect())
                    .unwrap_or_default();
                let column_fields = def
                    .column_fields
                    .as_ref()
                    .map(|cf| cf.field.iter().map(|f| name_of(f.index)).collect())
                    .unwrap_or_default();
                let filter_fields = def
                    .page_fields
                    .as_ref()
                    .map(|pf| pf.page_field.iter().map(|f| name_of(f.field)).collect())
                    .unwrap_or_default();
                let raw_data_fields: Vec<(i32, PivotAggregation, Option<String>, Option<u32>)> =
                    def.data_fields
                        .as_ref()
                        .map(|df| {
                            df.data_field
                                .iter()
                                .map(|f| {
                                    (
                                        f.field as i32,
                                        aggregation_from_sdk(f.subtotal),
                                        f.name.as_ref().map(|n| n.as_str().to_string()),
                                        f.number_format_id,
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                let data_fields: Vec<PivotDataField> = raw_data_fields
                    .into_iter()
                    .map(|(field, aggregation, name, num_fmt_id)| PivotDataField {
                        field: name_of(field),
                        aggregation,
                        name,
                        number_format: num_fmt_id
                            .and_then(|id| crate::styles::num_fmt_code(&mut self.doc, id)),
                    })
                    .collect();

                let source_ref = cache
                    .as_ref()
                    .and_then(|c| match &c.cache_source.cache_source_choice {
                        Some(x::CacheSourceChoice::WorksheetSource(ws)) => {
                            ws.reference.as_ref().map(|r| {
                                let prefix = ws
                                    .sheet
                                    .as_ref()
                                    .map(|s| format!("{}!", quote_sheet_name(s.as_str())))
                                    .unwrap_or_default();
                                format!("{prefix}{}", r.as_str())
                            })
                        }
                        _ => None,
                    })
                    .unwrap_or_default();

                let hidden_items: Option<Vec<PivotFieldFilter>> = def
                    .pivot_fields
                    .as_ref()
                    .map(|pf| {
                        pf.pivot_field
                            .iter()
                            .enumerate()
                            .filter_map(|(fi, f)| {
                                let items = f.items.as_ref()?;
                                let shared = cache
                                    .as_ref()?
                                    .cache_fields
                                    .cache_field
                                    .get(fi)?
                                    .shared_items
                                    .as_ref()?;
                                let hide: Vec<String> = items
                                    .item
                                    .iter()
                                    .filter(|it| it.hidden.map(Into::into).unwrap_or(false))
                                    .filter_map(|it| it.index.map(Into::into))
                                    .filter_map(|ix: u32| {
                                        shared
                                            .shared_items_choice
                                            .get(ix as usize)
                                            .and_then(shared_item_str)
                                    })
                                    .collect();
                                if hide.is_empty() {
                                    None
                                } else {
                                    Some(PivotFieldFilter {
                                        field: name_of(fi as i32),
                                        hide,
                                    })
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .filter(|v: &Vec<PivotFieldFilter>| !v.is_empty());

                let location_ref = def.location.reference.as_str().to_string();
                let n_page = def
                    .page_fields
                    .as_ref()
                    .map(|pf| pf.page_field.len() as u32)
                    .unwrap_or(0);
                let anchor_cell = anchor_from_location(&location_ref, n_page);

                out.push(PivotInfo {
                    sheet: sheet_name.clone(),
                    id,
                    name: def.name.as_str().to_string(),
                    location_ref,
                    anchor_cell,
                    source_ref,
                    row_fields,
                    column_fields,
                    filter_fields,
                    data_fields,
                    hidden_items,
                });
            }
        }
        Ok(out)
    }

    fn prepare_pivot(&mut self, patch: &PivotPatch) -> Result<PivotPrep> {
        if patch.data_fields.is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidPivot,
                "pivot must have at least one data field",
            )
            .with_sheet(&patch.sheet));
        }
        if patch.row_fields.is_empty() && patch.column_fields.is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidPivot,
                "pivot must have at least one row or column field",
            )
            .with_sheet(&patch.sheet));
        }
        if !self.sheet_exists(&patch.sheet)? {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {}", patch.sheet),
            )
            .with_sheet(&patch.sheet));
        }

        let anchor = self.resolve_cell_ref(&patch.anchor_cell)?;

        let source = self.get_range(&patch.source_ref)?;
        if source.rows < 2 {
            return Err(ApiError::new(
                ApiErrorCode::InvalidPivot,
                "pivot source must have a header row and at least one data row",
            )
            .with_sheet(&patch.sheet));
        }
        let source_sheet = source.sheet.clone();
        let source_a1 = format!(
            "{}:{}",
            cell_a1(source.start_row, source.start_column),
            cell_a1(source.end_row, source.end_column),
        );

        let headers: Vec<String> = source.values[0]
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let s = cell_to_string(v);
                if s.is_empty() {
                    format!("Column{}", i + 1)
                } else {
                    s
                }
            })
            .collect();
        let header_index: HashMap<&str, usize> = headers
            .iter()
            .enumerate()
            .map(|(i, h)| (h.as_str(), i))
            .collect();

        let role_of = |name: &str| -> Option<FieldRole> {
            if patch.row_fields.iter().any(|f| f == name) {
                Some(FieldRole::Row)
            } else if patch.column_fields.iter().any(|f| f == name) {
                Some(FieldRole::Column)
            } else if patch.filter_fields.iter().any(|f| f == name) {
                Some(FieldRole::Filter)
            } else if patch.data_fields.iter().any(|f| f.field == name) {
                Some(FieldRole::Data)
            } else {
                Some(FieldRole::Unused)
            }
        };

        for field in patch
            .row_fields
            .iter()
            .chain(patch.column_fields.iter())
            .chain(patch.filter_fields.iter())
            .chain(patch.data_fields.iter().map(|d| &d.field))
        {
            if !header_index.contains_key(field.as_str()) {
                return Err(ApiError::new(
                    ApiErrorCode::InvalidPivot,
                    format!("pivot field not found in source header: {field}"),
                )
                .with_sheet(&patch.sheet));
            }
        }

        let data_rows: Vec<Vec<CellValue>> = source.values[1..].to_vec();
        let mut columns: Vec<SourceColumn> = Vec::with_capacity(headers.len());
        for (col, name) in headers.iter().enumerate() {
            let role = role_of(name).unwrap();
            let mut numeric = true;
            let mut distinct: Vec<CellValue> = Vec::new();
            let mut distinct_index: HashMap<String, usize> = HashMap::new();
            for row in &data_rows {
                let v = row.get(col).unwrap_or(&CellValue::Blank);
                if !matches!(v, CellValue::Number(_) | CellValue::Blank) {
                    numeric = false;
                }
                if matches!(v, CellValue::Blank) {
                    continue;
                }
                let key = cell_to_string(v);
                if !distinct_index.contains_key(&key) {
                    distinct_index.insert(key.clone(), distinct.len());
                    distinct.push(v.clone());
                }
            }
            columns.push(SourceColumn {
                name: name.clone(),
                role,
                numeric,
                distinct,
                distinct_index,
            });
        }

        Ok(PivotPrep {
            columns,
            headers,
            anchor,
            source_sheet,
            source_a1,
            data_rows,
        })
    }

    pub fn pivot_preview(&mut self, patch: PivotPatch) -> Result<PivotGrid> {
        let prep = self.prepare_pivot(&patch)?;
        let cache_definition =
            build_cache_definition(&prep.source_sheet, &prep.source_a1, &prep.columns);
        let cache_records = build_cache_records(&prep.columns, &prep.data_rows);
        let name = patch
            .name
            .clone()
            .unwrap_or_else(|| "PivotPreview".to_string());
        let num_fmts = vec![None; patch.data_fields.len()];
        let definition = build_pivot_definition(
            &patch,
            &name,
            1,
            &prep.columns,
            &prep.headers,
            &prep.anchor,
            &num_fmts,
        );

        let mut styles = xlcore_export::Styles::default();
        let mut memo = None;
        let cells = xlcore_export::compute_cells(
            &definition,
            &cache_definition,
            &cache_records,
            &mut styles,
            &mut memo,
        );
        Ok(grid_from_cells(&cells, memo))
    }

    pub fn set_pivot(&mut self, patch: PivotPatch) -> Result<PivotInfo> {
        let prep = self.prepare_pivot(&patch)?;
        let columns = prep.columns;
        let headers = prep.headers;
        let anchor = prep.anchor;
        let data_rows = prep.data_rows;

        let cache_definition =
            build_cache_definition(&prep.source_sheet, &prep.source_a1, &columns);
        let cache_records = build_cache_records(&columns, &data_rows);
        let cache_id = self.next_pivot_cache_id()?;
        let name = patch
            .name
            .clone()
            .unwrap_or_else(|| format!("PivotTable{cache_id}"));
        let mut data_field_num_fmts = Vec::with_capacity(patch.data_fields.len());
        for d in &patch.data_fields {
            data_field_num_fmts.push(match &d.number_format {
                Some(code) => Some(crate::styles::resolve_num_fmt_id(&mut self.doc, code)?),
                None => None,
            });
        }
        let definition = build_pivot_definition(
            &patch,
            &name,
            cache_id,
            &columns,
            &headers,
            &anchor,
            &data_field_num_fmts,
        );

        let ws_part = self.worksheet_part_for_sheet(&patch.sheet)?;

        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let cache_def_part: PivotTableCacheDefinitionPart = wb_part
            .add_new_part_auto_id(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let cache_def_rid = cache_def_part
            .relationship_id()
            .ok_or_else(|| ApiError::new(ApiErrorCode::Other, "new cache def missing rid"))?
            .to_string();

        let records_part: PivotTableCacheRecordsPart = cache_def_part
            .add_new_part_auto_id(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let records_rid = records_part
            .relationship_id()
            .ok_or_else(|| ApiError::new(ApiErrorCode::Other, "new records missing rid"))?
            .to_string();
        records_part
            .set_root_element(&mut self.doc, cache_records)
            .map_err(sdk_err_to_api)?;

        let mut cache_definition = cache_definition;
        cache_definition.id = Some(records_rid);
        cache_def_part
            .set_root_element(&mut self.doc, cache_definition)
            .map_err(sdk_err_to_api)?;

        self.register_pivot_cache(cache_id, &cache_def_rid)?;

        let pivot_part: PivotTablePart = ws_part
            .add_new_part_auto_id(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let pivot_rid = pivot_part
            .relationship_id()
            .ok_or_else(|| ApiError::new(ApiErrorCode::Other, "new pivot missing rid"))?
            .to_string();
        pivot_part
            .set_root_element(&mut self.doc, definition)
            .map_err(sdk_err_to_api)?;
        pivot_part
            .create_relationship_to_part(&mut self.doc, cache_def_part)
            .map_err(sdk_err_to_api)?;

        self.pivots(Some(&patch.sheet))?
            .into_iter()
            .find(|p| p.id == pivot_rid)
            .ok_or_else(|| ApiError::new(ApiErrorCode::Other, "pivot not found after authoring"))
    }

    pub fn update_pivot(
        &mut self,
        sheet: impl AsRef<str>,
        id: impl AsRef<str>,
        update: PivotUpdate,
    ) -> Result<PivotInfo> {
        let sheet = sheet.as_ref().to_string();
        let id = id.as_ref().to_string();
        let existing = self
            .pivots(Some(&sheet))?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| {
                ApiError::new(
                    ApiErrorCode::Other,
                    format!("pivot not found on sheet '{sheet}': {id}"),
                )
                .with_sheet(&sheet)
            })?;

        let base = pivot_info_to_patch(&existing);
        let merged = PivotPatch {
            sheet: sheet.clone(),
            anchor_cell: update
                .anchor_cell
                .unwrap_or_else(|| base.anchor_cell.clone()),
            source_ref: update.source_ref.unwrap_or_else(|| base.source_ref.clone()),
            name: update.name.or_else(|| base.name.clone()),
            row_fields: update.row_fields.unwrap_or_else(|| base.row_fields.clone()),
            column_fields: update
                .column_fields
                .unwrap_or_else(|| base.column_fields.clone()),
            filter_fields: update
                .filter_fields
                .unwrap_or_else(|| base.filter_fields.clone()),
            data_fields: update
                .data_fields
                .unwrap_or_else(|| base.data_fields.clone()),
            hidden_items: update.hidden_items.or_else(|| base.hidden_items.clone()),
        };

        self.remove_pivot(&sheet, &id)?;
        match self.set_pivot(merged) {
            Ok(info) => Ok(info),
            Err(err) => {
                let _ = self.set_pivot(base);
                Err(err)
            }
        }
    }

    pub fn remove_pivot(
        &mut self,
        sheet: impl AsRef<str>,
        id: impl AsRef<str>,
    ) -> Result<Option<PivotInfo>> {
        let sheet = sheet.as_ref().to_string();
        let id = id.as_ref().to_string();
        let existing = self.pivots(Some(&sheet))?.into_iter().find(|p| p.id == id);
        let Some(info) = existing else {
            return Ok(None);
        };
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        ws_part
            .delete_part_by_id(&mut self.doc, &id)
            .map_err(sdk_err_to_api)?;
        Ok(Some(info))
    }

    fn next_pivot_cache_id(&mut self) -> Result<u32> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let wb = wb_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let max = wb
            .pivot_caches
            .as_ref()
            .map(|pc| pc.pivot_cache.iter().map(|c| c.cache_id).max().unwrap_or(0))
            .unwrap_or(0);
        Ok(max + 1)
    }

    fn register_pivot_cache(&mut self, cache_id: u32, rid: &str) -> Result<()> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let wb = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let caches = wb.pivot_caches.get_or_insert_with(Default::default);
        caches.pivot_cache.push(x::PivotCache {
            xml_other_attrs: Vec::new(),
            cache_id,
            id: rid.to_string(),
        });
        Ok(())
    }
}

fn anchor_from_location(location_ref: &str, n_page: u32) -> String {
    let top_left = location_ref.split(':').next().unwrap_or(location_ref);
    match xlcore_io::parse_a1(top_left) {
        Some((row, col)) => {
            let offset = if n_page > 0 { n_page + 1 } else { 0 };
            cell_a1(row.saturating_sub(offset).max(1), col)
        }
        None => top_left.to_string(),
    }
}

fn cell_a1(row: u32, col: u32) -> String {
    let mut name = String::new();
    let mut c = col;
    while c > 0 {
        let rem = ((c - 1) % 26) as u8;
        name.insert(0, (b'A' + rem) as char);
        c = (c - 1) / 26;
    }
    format!("{name}{row}")
}

fn cell_to_string(v: &CellValue) -> String {
    match v {
        CellValue::Blank => String::new(),
        CellValue::String(s) => s.clone(),
        CellValue::Number(n) => format_number(*n),
        CellValue::Boolean(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        CellValue::Error(e) => e.clone(),
    }
}

fn shared_item_str(c: &x::SharedItemsChoice) -> Option<String> {
    match c {
        x::SharedItemsChoice::NumberItem(b) => Some(format_number(b.val)),
        x::SharedItemsChoice::StringItem(b) => Some(b.val.clone()),
        x::SharedItemsChoice::BooleanItem(b) => {
            Some(if b.val.into() { "TRUE" } else { "FALSE" }.to_string())
        }
        _ => None,
    }
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        let s = format!("{n}");
        s
    }
}

fn aggregation_to_sdk(agg: PivotAggregation) -> DataConsolidateFunctionValues {
    match agg {
        PivotAggregation::Sum => DataConsolidateFunctionValues::Sum,
        PivotAggregation::Count => DataConsolidateFunctionValues::Count,
        PivotAggregation::Average => DataConsolidateFunctionValues::Average,
        PivotAggregation::Max => DataConsolidateFunctionValues::Maximum,
        PivotAggregation::Min => DataConsolidateFunctionValues::Minimum,
        PivotAggregation::Product => DataConsolidateFunctionValues::Product,
        PivotAggregation::CountNums => DataConsolidateFunctionValues::CountNumbers,
        PivotAggregation::StdDev => DataConsolidateFunctionValues::StandardDeviation,
        PivotAggregation::StdDevp => DataConsolidateFunctionValues::StandardDeviationP,
        PivotAggregation::Var => DataConsolidateFunctionValues::Variance,
        PivotAggregation::Varp => DataConsolidateFunctionValues::VarianceP,
    }
}

fn aggregation_from_sdk(v: Option<DataConsolidateFunctionValues>) -> PivotAggregation {
    match v {
        Some(DataConsolidateFunctionValues::Count) => PivotAggregation::Count,
        Some(DataConsolidateFunctionValues::Average) => PivotAggregation::Average,
        Some(DataConsolidateFunctionValues::Maximum) => PivotAggregation::Max,
        Some(DataConsolidateFunctionValues::Minimum) => PivotAggregation::Min,
        Some(DataConsolidateFunctionValues::Product) => PivotAggregation::Product,
        Some(DataConsolidateFunctionValues::CountNumbers) => PivotAggregation::CountNums,
        Some(DataConsolidateFunctionValues::StandardDeviation) => PivotAggregation::StdDev,
        Some(DataConsolidateFunctionValues::StandardDeviationP) => PivotAggregation::StdDevp,
        Some(DataConsolidateFunctionValues::Variance) => PivotAggregation::Var,
        Some(DataConsolidateFunctionValues::VarianceP) => PivotAggregation::Varp,
        _ => PivotAggregation::Sum,
    }
}

fn pivot_namespaces() -> Vec<ooxmlsdk::common::XmlNamespaceDecl> {
    vec![
        ooxmlsdk::common::XmlNamespaceDecl {
            prefix: "".into(),
            uri: SPREADSHEETML.into(),
        },
        ooxmlsdk::common::XmlNamespaceDecl {
            prefix: "r".into(),
            uri: RELATIONSHIPS.into(),
        },
    ]
}

fn build_cache_definition(
    source_sheet: &str,
    source_ref: &str,
    columns: &[SourceColumn],
) -> x::PivotCacheDefinition {
    let cache_fields: Vec<x::CacheField> = columns
        .iter()
        .map(|col| {
            let numeric = col.numeric && !col.distinct.is_empty();
            let mut items = x::SharedItems {
                count: Some(col.distinct.len() as u32),
                ..Default::default()
            };
            if numeric {
                let nums: Vec<f64> = col
                    .distinct
                    .iter()
                    .filter_map(|v| match v {
                        CellValue::Number(n) => Some(*n),
                        _ => None,
                    })
                    .collect();
                let min = nums.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let all_int = nums.iter().all(|n| n.fract() == 0.0);
                items.contains_string = Some(BooleanValue::from_bool(false));
                items.contains_semi_mixed_types = Some(BooleanValue::from_bool(false));
                items.contains_number = Some(BooleanValue::from_bool(true));
                if all_int {
                    items.contains_integer = Some(BooleanValue::from_bool(true));
                }
                if min.is_finite() {
                    items.min_value = Some(min);
                }
                if max.is_finite() {
                    items.max_value = Some(max);
                }
            } else {
                items.contains_string = Some(BooleanValue::from_bool(true));
                items.contains_semi_mixed_types = Some(BooleanValue::from_bool(true));
            }
            items.shared_items_choice = col
                .distinct
                .iter()
                .map(|v| match v {
                    CellValue::Number(n) if numeric => {
                        x::SharedItemsChoice::NumberItem(Box::new(x::NumberItem {
                            val: *n,
                            ..Default::default()
                        }))
                    }
                    other => x::SharedItemsChoice::StringItem(Box::new(x::StringItem {
                        val: cell_to_string(other),
                        ..Default::default()
                    })),
                })
                .collect();
            x::CacheField {
                xmlns: Vec::new(),
                xml_other_attrs: Vec::new(),
                name: col.name.clone(),
                shared_items: Some(items),
                ..Default::default()
            }
        })
        .collect();

    x::PivotCacheDefinition {
        xmlns: pivot_namespaces(),
        xml_header: crate::ooxml_header::STANDALONE,
        refresh_on_load: Some(BooleanValue::from_bool(true)),
        record_count: None,
        created_version: Some(8u8.into()),
        refreshed_version: Some(8u8.into()),
        min_refreshable_version: Some(3u8.into()),
        cache_source: Box::new(x::CacheSource {
            xml_other_attrs: Vec::new(),
            r#type: x::SourceValues::Worksheet,
            connection_id: None,
            cache_source_choice: Some(x::CacheSourceChoice::WorksheetSource(Box::new(
                x::WorksheetSource {
                    xml_other_attrs: Vec::new(),
                    reference: Some(source_ref.to_string()),
                    name: None,
                    sheet: Some(source_sheet.to_string()),
                    id: None,
                },
            ))),
        }),
        cache_fields: Box::new(x::CacheFields {
            count: Some(cache_fields.len() as u32),
            cache_field: cache_fields,
        }),
        ..Default::default()
    }
}

fn build_cache_records(
    columns: &[SourceColumn],
    data_rows: &[Vec<CellValue>],
) -> x::PivotCacheRecords {
    let records: Vec<x::PivotCacheRecord> = data_rows
        .iter()
        .map(|row| {
            let choices = columns
                .iter()
                .enumerate()
                .map(|(col_idx, col)| {
                    let v = row.get(col_idx).unwrap_or(&CellValue::Blank);
                    if matches!(v, CellValue::Blank) {
                        return x::PivotCacheRecordChoice::MissingItem(
                            Box::new(Default::default()),
                        );
                    }
                    let key = cell_to_string(v);
                    let idx = *col.distinct_index.get(&key).unwrap_or(&0);
                    x::PivotCacheRecordChoice::FieldItem(Box::new(x::FieldItem { val: idx as u32 }))
                })
                .collect();
            x::PivotCacheRecord {
                pivot_cache_record_choice: choices,
            }
        })
        .collect();

    x::PivotCacheRecords {
        xmlns: pivot_namespaces(),
        xml_header: crate::ooxml_header::STANDALONE,
        count: Some(records.len() as u32),
        pivot_cache_record: records,
        ..Default::default()
    }
}

fn build_pivot_definition(
    patch: &PivotPatch,
    name: &str,
    cache_id: u32,
    columns: &[SourceColumn],
    headers: &[String],
    anchor: &crate::refs::ResolvedCellRef,
    data_field_num_fmts: &[Option<u32>],
) -> x::PivotTableDefinition {
    let index_of =
        |field: &str| -> i32 { headers.iter().position(|h| h == field).unwrap_or(0) as i32 };

    let pivot_fields: Vec<x::PivotField> = columns
        .iter()
        .map(|col| {
            let mut pf = x::PivotField {
                compact: Some(BooleanValue::from_bool(false)),
                show_all: Some(BooleanValue::from_bool(false)),
                ..Default::default()
            };
            match col.role {
                FieldRole::Row => pf.axis = Some(PivotTableAxisValues::AxisRow),
                FieldRole::Column => pf.axis = Some(PivotTableAxisValues::AxisColumn),
                FieldRole::Filter => pf.axis = Some(PivotTableAxisValues::AxisPage),
                FieldRole::Data => {
                    pf.axis = Some(PivotTableAxisValues::AxisValues);
                    pf.data_field = Some(BooleanValue::from_bool(true));
                }
                FieldRole::Unused => {}
            }
            if matches!(
                col.role,
                FieldRole::Row | FieldRole::Column | FieldRole::Filter
            ) {
                pf.subtotal_top = Some(BooleanValue::from_bool(false));
                let hide: std::collections::HashSet<usize> = patch
                    .hidden_items
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .filter(|f| f.field == col.name)
                    .flat_map(|f| f.hide.iter())
                    .filter_map(|v| col.distinct_index.get(v).copied())
                    .collect();
                let mut items: Vec<x::Item> = (0..col.distinct.len())
                    .map(|i| x::Item {
                        index: Some(i as u32),
                        hidden: hide.contains(&i).then(|| BooleanValue::from_bool(true)),
                        ..Default::default()
                    })
                    .collect();
                items.push(x::Item {
                    item_type: Some(x::ItemValues::Default),
                    ..Default::default()
                });
                pf.items = Some(x::Items {
                    count: Some(items.len() as u32),
                    item: items,
                });
            }
            pf
        })
        .collect();

    let row_counts: Vec<usize> = patch
        .row_fields
        .iter()
        .map(|f| field_distinct(columns, f))
        .collect();
    let col_counts: Vec<usize> = patch
        .column_fields
        .iter()
        .map(|f| field_distinct(columns, f))
        .collect();
    let data_count = patch.data_fields.len();

    let row_fields = if patch.row_fields.is_empty() {
        None
    } else {
        Some(x::RowFields {
            count: Some(patch.row_fields.len() as u32),
            field: patch
                .row_fields
                .iter()
                .map(|f| x::Field { index: index_of(f) })
                .collect(),
        })
    };

    let mut col_field_entries: Vec<x::Field> = patch
        .column_fields
        .iter()
        .map(|f| x::Field { index: index_of(f) })
        .collect();
    if data_count > 1 || col_field_entries.is_empty() {
        col_field_entries.push(x::Field { index: -2 });
    }
    let column_fields = Some(x::ColumnFields {
        count: Some(col_field_entries.len() as u32),
        field: col_field_entries,
    });

    let row_items = build_axis_items(&row_counts, 1, false);
    let col_items = build_axis_items(&col_counts, data_count, true);

    let page_fields = if patch.filter_fields.is_empty() {
        None
    } else {
        Some(x::PageFields {
            count: Some(patch.filter_fields.len() as u32),
            page_field: patch
                .filter_fields
                .iter()
                .map(|f| x::PageField {
                    field: index_of(f),
                    item: None,
                    hierarchy: None,
                    name: None,
                    caption: None,
                    ..Default::default()
                })
                .collect(),
        })
    };

    let data_fields = x::DataFields {
        count: Some(patch.data_fields.len() as u32),
        data_field: patch
            .data_fields
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let header = &d.field;
                let default_name = format!("{} of {}", agg_label(d.aggregation), header);
                x::DataField {
                    xml_other_attrs: Vec::new(),
                    name: Some(d.name.clone().unwrap_or(default_name)),
                    field: index_of(header) as u32,
                    subtotal: Some(aggregation_to_sdk(d.aggregation)),
                    number_format_id: data_field_num_fmts.get(i).copied().flatten(),
                    ..Default::default()
                }
            })
            .collect(),
    };

    let n_page = patch.filter_fields.len() as u32;
    let n_col_levels = patch.column_fields.len() as u32 + 1;
    let location_ref = pivot_location_ref(anchor, columns, patch, n_page, n_col_levels);

    x::PivotTableDefinition {
        xmlns: pivot_namespaces(),
        xml_header: crate::ooxml_header::STANDALONE,
        name: name.to_string(),
        cache_id,
        data_caption: "Values".to_string(),
        apply_number_formats: Some(BooleanValue::from_bool(false)),
        apply_border_formats: Some(BooleanValue::from_bool(false)),
        apply_font_formats: Some(BooleanValue::from_bool(false)),
        apply_pattern_formats: Some(BooleanValue::from_bool(false)),
        apply_alignment_formats: Some(BooleanValue::from_bool(false)),
        apply_width_height_formats: Some(BooleanValue::from_bool(true)),
        use_auto_formatting: Some(BooleanValue::from_bool(true)),
        item_print_titles: Some(BooleanValue::from_bool(true)),
        created_version: Some(8u8.into()),
        updated_version: Some(8u8.into()),
        min_refreshable_version: Some(3u8.into()),
        indent: Some(0),
        compact: Some(BooleanValue::from_bool(false)),
        compact_data: Some(BooleanValue::from_bool(false)),
        outline: Some(BooleanValue::from_bool(true)),
        outline_data: Some(BooleanValue::from_bool(true)),
        multiple_field_filters: Some(BooleanValue::from_bool(false)),
        location: Box::new(x::Location {
            reference: location_ref,
            first_header_row: 1,
            first_data_row: n_col_levels,
            first_data_column: patch.row_fields.len().max(1) as u32,
            row_page_count: (n_page > 0).then_some(n_page),
            columns_per_page: (n_page > 0).then_some(1),
        }),
        pivot_fields: Some(x::PivotFields {
            count: Some(pivot_fields.len() as u32),
            pivot_field: pivot_fields,
        }),
        row_fields,
        row_items: (!row_items.is_empty()).then(|| x::RowItems {
            count: Some(row_items.len() as u32),
            row_item: row_items,
        }),
        column_fields,
        column_items: (!col_items.is_empty()).then(|| x::ColumnItems {
            count: Some(col_items.len() as u32),
            row_item: col_items,
        }),
        page_fields,
        data_fields: Some(data_fields),
        pivot_table_style: Some(x::PivotTableStyle {
            name: Some("PivotStyleLight16".to_string()),
            show_row_headers: Some(BooleanValue::from_bool(true)),
            show_column_headers: Some(BooleanValue::from_bool(true)),
            show_row_stripes: Some(BooleanValue::from_bool(false)),
            show_column_stripes: Some(BooleanValue::from_bool(false)),
            show_last_column: Some(BooleanValue::from_bool(true)),
        }),
        ..Default::default()
    }
}

fn pivot_location_ref(
    anchor: &crate::refs::ResolvedCellRef,
    columns: &[SourceColumn],
    patch: &PivotPatch,
    n_page: u32,
    n_col_levels: u32,
) -> String {
    let row_combos: u32 = columns
        .iter()
        .filter(|c| matches!(c.role, FieldRole::Row))
        .map(|c| c.distinct.len().max(1) as u32)
        .product::<u32>()
        .max(1);
    let col_combos: u32 = columns
        .iter()
        .filter(|c| matches!(c.role, FieldRole::Column))
        .map(|c| c.distinct.len().max(1) as u32)
        .product::<u32>()
        .max(1);

    let top = anchor.row + if n_page > 0 { n_page + 1 } else { 0 };
    let header_rows = n_col_levels;
    let total_rows = header_rows + row_combos + 1;
    let total_cols =
        patch.row_fields.len().max(1) as u32 + col_combos * patch.data_fields.len() as u32 + 1;

    let start = cell_a1(top, anchor.column);
    let end = cell_a1(top + total_rows - 1, anchor.column + total_cols - 1);
    format!("{start}:{end}")
}

fn field_distinct(columns: &[SourceColumn], name: &str) -> usize {
    columns
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.distinct.len())
        .unwrap_or(0)
}

fn cartesian(counts: &[usize]) -> Vec<Vec<u32>> {
    let mut out: Vec<Vec<u32>> = vec![Vec::new()];
    for &n in counts {
        let mut next = Vec::new();
        for prefix in &out {
            for i in 0..n {
                let mut t = prefix.clone();
                t.push(i as u32);
                next.push(t);
            }
        }
        out = next;
    }
    out
}

fn common_prefix(a: &[u32], b: &[u32]) -> usize {
    a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count()
}

fn x_index(v: u32) -> x::MemberPropertyIndex {
    x::MemberPropertyIndex {
        val: (v > 0).then_some(v as i32),
    }
}

fn build_axis_items(counts: &[usize], data_count: usize, is_value_axis: bool) -> Vec<x::RowItem> {
    if counts.iter().any(|&n| n == 0) {
        return Vec::new();
    }
    let tuples = cartesian(counts);
    let mut items: Vec<x::RowItem> = Vec::new();
    let mut prev: Vec<u32> = Vec::new();
    let multi_data = is_value_axis && data_count > 1;

    for t in &tuples {
        let r = common_prefix(&prev, t);
        let changed: Vec<x::MemberPropertyIndex> = t[r..].iter().map(|&v| x_index(v)).collect();
        let reps = if is_value_axis { data_count.max(1) } else { 1 };
        for d in 0..reps {
            let (rep, xs) = if d == 0 {
                (
                    (r > 0).then_some(r as u32),
                    if changed.is_empty() {
                        vec![x_index(0)]
                    } else {
                        changed.clone()
                    },
                )
            } else {
                (Some(t.len() as u32), Vec::new())
            };
            items.push(x::RowItem {
                item_type: None,
                repeated_item_count: rep,
                index: multi_data.then_some(d as u32),
                member_property_index: xs,
            });
        }
        prev = t.clone();
    }

    let want_grand = !counts.is_empty();
    if want_grand {
        let grand_reps = if is_value_axis { data_count.max(1) } else { 1 };
        for d in 0..grand_reps {
            items.push(x::RowItem {
                item_type: Some(x::ItemValues::Grand),
                repeated_item_count: None,
                index: multi_data.then_some(d as u32),
                member_property_index: vec![x::MemberPropertyIndex { val: None }],
            });
        }
    }
    items
}

fn agg_label(agg: PivotAggregation) -> &'static str {
    match agg {
        PivotAggregation::Sum => "Sum",
        PivotAggregation::Count => "Count",
        PivotAggregation::Average => "Average",
        PivotAggregation::Max => "Max",
        PivotAggregation::Min => "Min",
        PivotAggregation::Product => "Product",
        PivotAggregation::CountNums => "Count",
        PivotAggregation::StdDev => "StdDev",
        PivotAggregation::StdDevp => "StdDevp",
        PivotAggregation::Var => "Var",
        PivotAggregation::Varp => "Varp",
    }
}
