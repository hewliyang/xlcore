use std::collections::{BTreeSet, HashMap};

use ironcalc_base::expressions::parser::stringify::to_excel_string;
use ironcalc_base::expressions::parser::{new_parser_english, DefinedNameS, Node, Parser};
use ironcalc_base::expressions::types::CellReferenceRC;
use xlcore_types::{ApiError, ApiErrorCode, DependencyInfo, DependencyReference};

use crate::errors::sdk_err_to_api;
use crate::refs::{qualify_ref, ranges_overlap, ResolvedCellRef, ResolvedRangeRef};
use crate::{Result, Workbook};

impl Workbook {
    pub fn dependencies_in(&mut self, sheet: &str, reference: &str) -> Result<DependencyInfo> {
        let reference = qualify_ref(sheet, reference)?;
        self.dependencies(reference)
    }

    pub fn precedents_in(
        &mut self,
        sheet: &str,
        reference: &str,
    ) -> Result<Vec<DependencyReference>> {
        let reference = qualify_ref(sheet, reference)?;
        self.precedents(reference)
    }

    pub fn parse_formula_references_in(
        &mut self,
        sheet: &str,
        anchor: &str,
        formula: &str,
    ) -> Result<Vec<DependencyReference>> {
        let cell_ref = self.resolve_cell_ref(&qualify_ref(sheet, anchor)?)?;
        self.references_for_formula(&cell_ref, formula)
    }

    pub fn function_names(&mut self) -> Result<Vec<String>> {
        Ok(ironcalc_base::english_function_names())
    }

    pub fn dependents_in(
        &mut self,
        sheet: &str,
        reference: &str,
    ) -> Result<Vec<DependencyReference>> {
        let reference = qualify_ref(sheet, reference)?;
        self.dependents(reference)
    }

    pub fn dependencies(&mut self, reference: impl AsRef<str>) -> Result<DependencyInfo> {
        let cell_ref = self.resolve_cell_ref(reference.as_ref())?;
        let precedents = self.precedents(cell_ref.full_reference())?;
        let dependents = self.dependents(cell_ref.full_reference())?;
        let reference = cell_ref.cell_reference();
        Ok(DependencyInfo {
            sheet: cell_ref.sheet,
            reference,
            row: cell_ref.row,
            column: cell_ref.column,
            precedents,
            dependents,
        })
    }

    pub fn precedents(&mut self, reference: impl AsRef<str>) -> Result<Vec<DependencyReference>> {
        let cell_ref = self.resolve_cell_ref(reference.as_ref())?;
        let info = self.get_cell(cell_ref.full_reference())?;
        let Some(formula) = info.formula else {
            return Ok(Vec::new());
        };
        self.references_for_formula(&cell_ref, &formula)
    }

    pub(crate) fn formula_parser(&mut self) -> Result<Parser<'static>> {
        let context = self.dependency_context()?;
        Ok(new_parser_english(
            context.sheet_names,
            context.defined_names,
            HashMap::new(),
        ))
    }

    pub(crate) fn canonicalize_formula(
        &mut self,
        cell_ref: &ResolvedCellRef,
        formula: &str,
    ) -> Result<String> {
        let mut parser = self.formula_parser()?;
        Ok(canonicalize_with_parser(
            &mut parser,
            &cell_ref.sheet,
            cell_ref.row,
            cell_ref.column,
            formula,
        ))
    }

    fn references_for_formula(
        &mut self,
        cell_ref: &ResolvedCellRef,
        formula: &str,
    ) -> Result<Vec<DependencyReference>> {
        let context = self.dependency_context()?;
        let sheet_index = sheet_index_for(&context, &cell_ref.sheet)?;
        Ok(parse_formula_references(
            formula,
            FormulaContext {
                sheet_index,
                row: cell_ref.row as i32,
                column: cell_ref.column as i32,
            },
            &context,
        ))
    }

    pub fn dependents(&mut self, reference: impl AsRef<str>) -> Result<Vec<DependencyReference>> {
        let range_ref = self.resolve_range_ref(reference.as_ref())?;
        let context = self.dependency_context()?;
        let target = target_key(&context, &range_ref)?;
        let formulas = self.formula_cells(&context.sheet_names)?;
        let mut out = BTreeSet::new();

        for formula in formulas {
            let precedents = parse_formula_reference_keys(
                &formula.formula,
                FormulaContext {
                    sheet_index: formula.sheet_index,
                    row: formula.row as i32,
                    column: formula.column as i32,
                },
                &context,
            );
            if precedents
                .iter()
                .any(|precedent| dependency_ranges_overlap(precedent, &target))
            {
                out.insert(DependencyKey {
                    sheet_index: formula.sheet_index,
                    start_row: formula.row,
                    start_column: formula.column,
                    end_row: formula.row,
                    end_column: formula.column,
                });
            }
        }

        Ok(out
            .into_iter()
            .filter_map(|key| dependency_reference(&context, key))
            .collect())
    }

    fn dependency_context(&mut self) -> Result<DependencyContext> {
        let sheet_names = self
            .workbook_sheets()?
            .into_iter()
            .map(|sheet| sheet.name.as_str().to_string())
            .collect::<Vec<_>>();
        let defined_names = self
            .defined_names()?
            .into_iter()
            .map(|name| {
                let scope = name
                    .scope
                    .as_ref()
                    .and_then(|sheet| sheet_names.iter().position(|s| s == sheet))
                    .map(|index| index as u32);
                (name.name, scope, name.reference)
            })
            .collect();
        Ok(DependencyContext {
            sheet_names,
            defined_names,
        })
    }

    fn formula_cells(&mut self, sheet_names: &[String]) -> Result<Vec<FormulaCell>> {
        let mut out = Vec::new();
        for (sheet_index, sheet_name) in sheet_names.iter().enumerate() {
            let ws_part = self.worksheet_part_for_sheet(sheet_name)?;
            let ws = ws_part
                .root_element(&mut self.doc)
                .map_err(sdk_err_to_api)?;
            for row in &ws.sheet_data.row {
                if row.row_index.is_none() {
                    continue;
                }
                for cell in &row.cell {
                    let Some(formula) = cell
                        .cell_formula
                        .as_ref()
                        .and_then(|formula| formula.xml_content.as_deref())
                    else {
                        continue;
                    };
                    let Some((r, c)) = cell
                        .cell_reference
                        .as_ref()
                        .and_then(|reference| xlcore_io::parse_a1(reference.as_str()))
                    else {
                        continue;
                    };
                    out.push(FormulaCell {
                        sheet_index: sheet_index as u32,
                        row: r,
                        column: c,
                        formula: formula.to_string(),
                    });
                }
            }
        }
        Ok(out)
    }
}

struct DependencyContext {
    sheet_names: Vec<String>,
    defined_names: Vec<DefinedNameS>,
}

#[derive(Clone, Copy)]
struct FormulaContext {
    sheet_index: u32,
    row: i32,
    column: i32,
}

struct FormulaCell {
    sheet_index: u32,
    row: u32,
    column: u32,
    formula: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DependencyKey {
    sheet_index: u32,
    start_row: u32,
    start_column: u32,
    end_row: u32,
    end_column: u32,
}

pub(crate) fn canonicalize_with_parser(
    parser: &mut Parser,
    sheet: &str,
    row: u32,
    column: u32,
    formula: &str,
) -> String {
    let stripped = formula.trim().strip_prefix('=').unwrap_or(formula.trim());
    if stripped.is_empty() {
        return stripped.to_string();
    }
    let cell_ref_rc = CellReferenceRC {
        sheet: sheet.to_string(),
        row: row as i32,
        column: column as i32,
    };
    let node = parser.parse(stripped, &cell_ref_rc);
    if matches!(node, Node::ParseErrorKind { .. }) {
        return stripped.to_string();
    }
    to_excel_string(&node, &cell_ref_rc)
}

fn parse_formula_references(
    formula: &str,
    formula_context: FormulaContext,
    context: &DependencyContext,
) -> Vec<DependencyReference> {
    parse_formula_reference_keys(formula, formula_context, context)
        .into_iter()
        .filter_map(|key| dependency_reference(context, key))
        .collect()
}

fn parse_formula_reference_keys(
    formula: &str,
    formula_context: FormulaContext,
    context: &DependencyContext,
) -> BTreeSet<DependencyKey> {
    let mut out = BTreeSet::new();
    let mut seen_names = BTreeSet::new();
    collect_formula_references(formula, formula_context, context, &mut out, &mut seen_names);
    out
}

fn collect_formula_references(
    formula: &str,
    formula_context: FormulaContext,
    context: &DependencyContext,
    out: &mut BTreeSet<DependencyKey>,
    seen_names: &mut BTreeSet<(String, Option<u32>)>,
) {
    let Some(sheet) = context
        .sheet_names
        .get(formula_context.sheet_index as usize)
        .cloned()
    else {
        return;
    };
    let formula = formula.trim();
    let formula = formula.strip_prefix('=').unwrap_or(formula);
    let mut parser = new_parser_english(
        context.sheet_names.clone(),
        context.defined_names.clone(),
        HashMap::new(),
    );
    let node = parser.parse(
        formula,
        &CellReferenceRC {
            sheet,
            row: formula_context.row,
            column: formula_context.column,
        },
    );
    collect_node_references(&node, formula_context, context, out, seen_names);
}

fn collect_node_references(
    node: &Node,
    formula_context: FormulaContext,
    context: &DependencyContext,
    out: &mut BTreeSet<DependencyKey>,
    seen_names: &mut BTreeSet<(String, Option<u32>)>,
) {
    match node {
        Node::ReferenceKind { .. } | Node::RangeKind { .. } => {
            if let Some(key) = area_from_node(node, formula_context) {
                out.insert(key);
            }
        }
        Node::OpRangeKind { left, right } => {
            if let Some(key) = area_from_node(node, formula_context) {
                out.insert(key);
            } else {
                collect_node_references(left, formula_context, context, out, seen_names);
                collect_node_references(right, formula_context, context, out, seen_names);
            }
        }
        Node::OpConcatenateKind { left, right }
        | Node::OpSumKind { left, right, .. }
        | Node::OpProductKind { left, right, .. }
        | Node::OpPowerKind { left, right }
        | Node::CompareKind { left, right, .. } => {
            collect_node_references(left, formula_context, context, out, seen_names);
            collect_node_references(right, formula_context, context, out, seen_names);
        }
        Node::UnaryKind { right, .. } => {
            collect_node_references(right, formula_context, context, out, seen_names);
        }
        Node::FunctionKind { args, .. } | Node::InvalidFunctionKind { args, .. } => {
            for arg in args {
                collect_node_references(arg, formula_context, context, out, seen_names);
            }
        }
        Node::ImplicitIntersection { child, .. } => {
            collect_node_references(child, formula_context, context, out, seen_names);
        }
        Node::DefinedNameKind((name, scope, formula)) => {
            let key = (name.to_ascii_uppercase(), *scope);
            if seen_names.insert(key) {
                collect_formula_references(
                    formula,
                    FormulaContext {
                        sheet_index: scope.unwrap_or(formula_context.sheet_index),
                        row: formula_context.row,
                        column: formula_context.column,
                    },
                    context,
                    out,
                    seen_names,
                );
            }
        }
        _ => {}
    }
}

fn area_from_node(node: &Node, formula_context: FormulaContext) -> Option<DependencyKey> {
    match node {
        Node::ReferenceKind {
            sheet_index,
            absolute_row,
            absolute_column,
            row,
            column,
            ..
        } => Some(DependencyKey {
            sheet_index: *sheet_index,
            start_row: resolve_coordinate(*row, formula_context.row, *absolute_row)?,
            start_column: resolve_coordinate(*column, formula_context.column, *absolute_column)?,
            end_row: resolve_coordinate(*row, formula_context.row, *absolute_row)?,
            end_column: resolve_coordinate(*column, formula_context.column, *absolute_column)?,
        }),
        Node::RangeKind {
            sheet_index,
            absolute_row1,
            absolute_column1,
            row1,
            column1,
            absolute_row2,
            absolute_column2,
            row2,
            column2,
            ..
        } => normalize_area(
            *sheet_index,
            resolve_coordinate(*row1, formula_context.row, *absolute_row1)?,
            resolve_coordinate(*column1, formula_context.column, *absolute_column1)?,
            resolve_coordinate(*row2, formula_context.row, *absolute_row2)?,
            resolve_coordinate(*column2, formula_context.column, *absolute_column2)?,
        ),
        Node::OpRangeKind { left, right } => {
            let left = area_from_node(left, formula_context)?;
            let right = area_from_node(right, formula_context)?;
            (left.sheet_index == right.sheet_index)
                .then(|| {
                    normalize_area(
                        left.sheet_index,
                        left.start_row.min(right.start_row),
                        left.start_column.min(right.start_column),
                        left.end_row.max(right.end_row),
                        left.end_column.max(right.end_column),
                    )
                })
                .flatten()
        }
        _ => None,
    }
}

fn resolve_coordinate(value: i32, base: i32, absolute: bool) -> Option<u32> {
    let resolved = if absolute { value } else { base + value };
    (resolved > 0).then_some(resolved as u32)
}

fn normalize_area(
    sheet_index: u32,
    row1: u32,
    column1: u32,
    row2: u32,
    column2: u32,
) -> Option<DependencyKey> {
    Some(DependencyKey {
        sheet_index,
        start_row: row1.min(row2),
        start_column: column1.min(column2),
        end_row: row1.max(row2),
        end_column: column1.max(column2),
    })
}

fn dependency_reference(
    context: &DependencyContext,
    key: DependencyKey,
) -> Option<DependencyReference> {
    let sheet = context.sheet_names.get(key.sheet_index as usize)?.clone();
    Some(DependencyReference {
        sheet,
        reference: range_reference(key.start_row, key.start_column, key.end_row, key.end_column),
        start_row: key.start_row,
        start_column: key.start_column,
        end_row: key.end_row,
        end_column: key.end_column,
        rows: key.end_row - key.start_row + 1,
        columns: key.end_column - key.start_column + 1,
    })
}

fn range_reference(start_row: u32, start_column: u32, end_row: u32, end_column: u32) -> String {
    let start = format!("{}{}", xlcore_io::col_label(start_column), start_row);
    let end = format!("{}{}", xlcore_io::col_label(end_column), end_row);
    if start == end {
        start
    } else {
        format!("{start}:{end}")
    }
}

fn sheet_index_for(context: &DependencyContext, sheet: &str) -> Result<u32> {
    context
        .sheet_names
        .iter()
        .position(|name| name == sheet)
        .map(|index| index as u32)
        .ok_or_else(|| {
            ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {sheet}"),
            )
            .with_sheet(sheet)
        })
}

fn target_key(context: &DependencyContext, range_ref: &ResolvedRangeRef) -> Result<DependencyKey> {
    Ok(DependencyKey {
        sheet_index: sheet_index_for(context, &range_ref.sheet)?,
        start_row: range_ref.start_row,
        start_column: range_ref.start_column,
        end_row: range_ref.end_row,
        end_column: range_ref.end_column,
    })
}

fn dependency_ranges_overlap(a: &DependencyKey, b: &DependencyKey) -> bool {
    a.sheet_index == b.sheet_index
        && ranges_overlap(
            a.start_row,
            a.start_column,
            a.end_row,
            a.end_column,
            b.start_row,
            b.start_column,
            b.end_row,
            b.end_column,
        )
}
