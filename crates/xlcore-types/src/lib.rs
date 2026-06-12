mod annotations;
mod cells;
mod charts;
mod drawings;
mod engine;
mod errors;
mod names;
mod page_setup;
mod pivots;
mod properties;
mod protection;
mod sparklines;
mod styles;
mod tables;
mod validation;

pub use annotations::*;
pub use cells::*;
pub use charts::*;
pub use drawings::*;
pub use engine::*;
pub use errors::*;
pub use names::*;
pub use page_setup::*;
pub use pivots::*;
pub use properties::*;
pub use protection::*;
pub use sparklines::*;
pub use styles::*;
pub use tables::*;
pub use validation::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recalc_workbook_finds_cells_by_a1_reference() {
        let workbook = RecalcWorkbook {
            sheets: vec![RecalcSheet {
                index: 0,
                name: "Sheet1".to_string(),
                cells: vec![RecalcCell {
                    r: 2,
                    c: 3,
                    formula: "A1+B1".to_string(),
                    cached_value: None,
                    value: EngineCellValue::Number(7.0),
                    fallback: None,
                }],
            }],
        };

        assert_eq!(
            workbook.cell("Sheet1", "C2").map(|cell| &cell.value),
            Some(&EngineCellValue::Number(7.0))
        );
        assert!(workbook.cell("Sheet1", "2C").is_none());
    }

    #[test]
    fn clear_mode_accepts_formats_alias_for_styles() {
        let styles: ClearMode = serde_json::from_str("\"styles\"").unwrap();
        let formats: ClearMode = serde_json::from_str("\"formats\"").unwrap();
        assert_eq!(styles, ClearMode::Styles);
        assert_eq!(formats, ClearMode::Styles);
    }
}
