use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    CellInfo, FontPatch, FontScheme, RichText, RichTextRun, UnderlinePatch, VertAlign,
};

use crate::errors::sdk_err_to_api;
use crate::refs::qualify_ref;
use crate::styles::parse_color;
use crate::xml::ensure_cell;
use crate::{ApiError, ApiErrorCode, Result, Workbook};

impl Workbook {
    pub fn set_rich_text_in(
        &mut self,
        sheet: &str,
        reference: &str,
        runs: Vec<RichTextRun>,
    ) -> Result<CellInfo> {
        let reference = qualify_ref(sheet, reference)?;
        self.set_rich_text(reference, runs)
    }

    pub fn set_rich_text(
        &mut self,
        reference: impl AsRef<str>,
        runs: Vec<RichTextRun>,
    ) -> Result<CellInfo> {
        let cell_ref = self.resolve_cell_ref(reference.as_ref())?;
        if runs.is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::Other,
                "rich text requires at least one run",
            ));
        }
        let mut sdk_runs = Vec::with_capacity(runs.len());
        for run in &runs {
            sdk_runs.push(build_run(run)?);
        }
        let ws_part = self.worksheet_part_for_sheet(&cell_ref.sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let cell = ensure_cell(ws, cell_ref.row, cell_ref.column);
        cell.cell_formula = None;
        cell.cell_value = None;
        cell.data_type = Some(x::CellValues::InlineString);
        cell.inline_string = Some(Box::new(x::InlineString {
            run: sdk_runs,
            ..Default::default()
        }));
        self.mark_formulas_stale()?;
        self.get_cell(cell_ref.full_reference())
    }
}

fn build_run(run: &RichTextRun) -> Result<x::Run> {
    Ok(x::Run {
        run_properties: run
            .font
            .as_ref()
            .map(build_run_properties)
            .transpose()?
            .flatten(),
        text: Box::new(x::Text {
            xml_content: Some(run.text.clone()),
            ..Default::default()
        }),
    })
}

fn build_run_properties(patch: &FontPatch) -> Result<Option<x::RunProperties>> {
    let mut choices: Vec<x::RunPropertiesChoice> = Vec::new();
    if let Some(name) = patch.name.as_deref() {
        choices.push(x::RunPropertiesChoice::RunFont(Box::new(x::RunFont {
            val: name.to_string(),
        })));
    }
    if patch.bold == Some(true) {
        choices.push(x::RunPropertiesChoice::Bold(Box::new(x::Bold {
            val: None,
        })));
    }
    if patch.italic == Some(true) {
        choices.push(x::RunPropertiesChoice::Italic(Box::new(x::Italic {
            val: None,
        })));
    }
    if patch.strike == Some(true) {
        choices.push(x::RunPropertiesChoice::Strike(Box::new(x::Strike {
            val: None,
        })));
    }
    if let Some(underline) = patch.underline {
        match underline {
            UnderlinePatch::None => {}
            UnderlinePatch::Single => {
                choices.push(x::RunPropertiesChoice::Underline(Box::new(x::Underline {
                    val: None,
                })));
            }
            UnderlinePatch::Double => {
                choices.push(x::RunPropertiesChoice::Underline(Box::new(x::Underline {
                    val: Some(x::UnderlineValues::Double),
                })));
            }
        }
    }
    if let Some(vert) = patch.vert_align {
        if !matches!(vert, VertAlign::Baseline) {
            choices.push(x::RunPropertiesChoice::VerticalTextAlignment(Box::new(
                x::VerticalTextAlignment {
                    val: match vert {
                        VertAlign::Baseline => x::VerticalAlignmentRunValues::Baseline,
                        VertAlign::Superscript => x::VerticalAlignmentRunValues::Superscript,
                        VertAlign::Subscript => x::VerticalAlignmentRunValues::Subscript,
                    },
                },
            )));
        }
    }
    if let Some(size) = patch.size {
        choices.push(x::RunPropertiesChoice::FontSize(Box::new(x::FontSize {
            val: size,
        })));
    }
    if let Some(color) = patch.color.as_deref() {
        choices.push(x::RunPropertiesChoice::Color(Box::new(parse_color(color)?)));
    }
    if let Some(family) = patch.family {
        choices.push(x::RunPropertiesChoice::FontFamily(Box::new(
            x::FontFamily { val: family as i32 },
        )));
    }
    if let Some(scheme) = patch.scheme {
        choices.push(x::RunPropertiesChoice::FontScheme(Box::new(
            x::FontScheme {
                val: match scheme {
                    FontScheme::None => x::FontSchemeValues::None,
                    FontScheme::Major => x::FontSchemeValues::Major,
                    FontScheme::Minor => x::FontSchemeValues::Minor,
                },
            },
        )));
    }
    if choices.is_empty() {
        return Ok(None);
    }
    Ok(Some(x::RunProperties {
        run_properties_choice: choices,
    }))
}

pub(crate) fn rich_text_from_cell(cell: &x::Cell) -> Option<RichText> {
    let inline = cell.inline_string.as_ref()?;
    if inline.run.is_empty() {
        return None;
    }
    let runs = inline
        .run
        .iter()
        .map(|r| RichTextRun {
            text: r.text.xml_content.clone().unwrap_or_default(),
            font: r.run_properties.as_ref().and_then(run_properties_to_font),
        })
        .collect();
    Some(RichText { runs })
}

fn run_properties_to_font(rpr: &x::RunProperties) -> Option<FontPatch> {
    let mut font = FontPatch::default();
    for choice in &rpr.run_properties_choice {
        match choice {
            x::RunPropertiesChoice::Bold(b) => {
                font.bold = Some(b.val.map(bool::from).unwrap_or(true));
            }
            x::RunPropertiesChoice::Italic(i) => {
                font.italic = Some(i.val.map(bool::from).unwrap_or(true));
            }
            x::RunPropertiesChoice::Strike(s) => {
                font.strike = Some(s.val.map(bool::from).unwrap_or(true));
            }
            x::RunPropertiesChoice::Underline(u) => {
                font.underline = Some(match u.val {
                    Some(x::UnderlineValues::None) => UnderlinePatch::None,
                    Some(x::UnderlineValues::Double)
                    | Some(x::UnderlineValues::DoubleAccounting) => UnderlinePatch::Double,
                    _ => UnderlinePatch::Single,
                });
            }
            x::RunPropertiesChoice::VerticalTextAlignment(v) => {
                font.vert_align = Some(match v.val {
                    x::VerticalAlignmentRunValues::Superscript => VertAlign::Superscript,
                    x::VerticalAlignmentRunValues::Subscript => VertAlign::Subscript,
                    x::VerticalAlignmentRunValues::Baseline => VertAlign::Baseline,
                });
            }
            x::RunPropertiesChoice::FontSize(s) => font.size = Some(s.val),
            x::RunPropertiesChoice::Color(c) => {
                if let Some(rgb) = c.rgb.as_deref() {
                    font.color = Some(format!("#{rgb}"));
                }
            }
            x::RunPropertiesChoice::RunFont(n) => font.name = Some(n.val.as_str().to_string()),
            x::RunPropertiesChoice::FontFamily(fm) => {
                if (0..=14).contains(&fm.val) {
                    font.family = Some(fm.val as u32);
                }
            }
            x::RunPropertiesChoice::FontScheme(s) => {
                font.scheme = Some(match s.val {
                    x::FontSchemeValues::Major => FontScheme::Major,
                    x::FontSchemeValues::Minor => FontScheme::Minor,
                    x::FontSchemeValues::None => FontScheme::None,
                });
            }
            _ => {}
        }
    }
    if font == FontPatch::default() {
        None
    } else {
        Some(font)
    }
}
