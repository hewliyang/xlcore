use crate::*;

fn search_fixture() -> Workbook {
    let mut wb = Workbook::new().unwrap();
    wb.create_sheet("Inputs").unwrap();
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!A2", "North").unwrap();
    wb.set_value("Sheet1!A3", "NORTHEAST").unwrap();
    wb.set_value("Sheet1!A4", 42.0).unwrap();
    wb.set_value("Sheet1!A5", true).unwrap();
    wb.set_formula("Sheet1!B2", "=SUM(A4:A4)").unwrap();
    wb.set_value("Inputs!A1", "north pole").unwrap();
    wb.set_formula("Inputs!B1", "=AVERAGE(Sheet1!A4:A4)")
        .unwrap();
    wb
}

#[test]
fn search_substring_default_case_insensitive_across_sheets() {
    let mut wb = search_fixture();
    let hits = wb.search("north", SearchOptions::default()).unwrap();
    let refs: Vec<_> = hits
        .iter()
        .map(|m| (m.sheet.as_str(), m.reference.as_str(), m.hit))
        .collect();
    assert_eq!(
        refs,
        vec![
            ("Sheet1", "Sheet1!A2", SearchHit::Value),
            ("Sheet1", "Sheet1!A3", SearchHit::Value),
            ("Inputs", "Inputs!A1", SearchHit::Value),
        ],
    );
    assert_eq!(hits[0].matched, "North");
}

#[test]
fn search_case_sensitive_narrows_results() {
    let mut wb = search_fixture();
    let hits = wb
        .search(
            "NORTH",
            SearchOptions {
                case_sensitive: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].reference, "Sheet1!A3");
}

#[test]
fn search_exact_mode_requires_full_cell() {
    let mut wb = search_fixture();
    let hits = wb
        .search(
            "North",
            SearchOptions {
                mode: SearchMode::Exact,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].reference, "Sheet1!A2");
}

#[test]
fn search_formulas_target_only_matches_formula_text() {
    let mut wb = search_fixture();
    let hits = wb
        .search(
            "SUM",
            SearchOptions {
                target: SearchTarget::Formulas,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].reference, "Sheet1!B2");
    assert_eq!(hits[0].hit, SearchHit::Formula);
    assert_eq!(hits[0].formula.as_deref(), Some("SUM(A4:A4)"));
}

#[test]
fn search_both_target_returns_separate_hits_per_cell() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "total").unwrap();
    wb.set_formula("Sheet1!A2", "=total").unwrap();
    let hits = wb
        .search(
            "total",
            SearchOptions {
                target: SearchTarget::Both,
                ..Default::default()
            },
        )
        .unwrap();
    let kinds: Vec<_> = hits.iter().map(|h| (h.reference.as_str(), h.hit)).collect();
    assert_eq!(
        kinds,
        vec![
            ("Sheet1!A1", SearchHit::Value),
            ("Sheet1!A2", SearchHit::Formula),
        ],
    );
}

#[test]
fn search_wildcard_anchors_full_cell() {
    let mut wb = search_fixture();
    let hits = wb
        .search(
            "north*",
            SearchOptions {
                mode: SearchMode::Wildcard,
                ..Default::default()
            },
        )
        .unwrap();
    let refs: Vec<_> = hits.iter().map(|m| m.reference.as_str()).collect();
    assert_eq!(refs, vec!["Sheet1!A2", "Sheet1!A3", "Inputs!A1"]);

    let hits = wb
        .search(
            "north",
            SearchOptions {
                mode: SearchMode::Wildcard,
                ..Default::default()
            },
        )
        .unwrap();
    let refs: Vec<_> = hits.iter().map(|m| m.reference.as_str()).collect();
    assert_eq!(refs, vec!["Sheet1!A2"]);
}

#[test]
fn search_regex_mode_and_invalid_pattern_diagnosed() {
    let mut wb = search_fixture();
    let hits = wb
        .search(
            r"^N\w+$",
            SearchOptions {
                mode: SearchMode::Regex,
                case_sensitive: true,
                ..Default::default()
            },
        )
        .unwrap();
    let refs: Vec<_> = hits.iter().map(|m| m.reference.as_str()).collect();
    assert_eq!(refs, vec!["Sheet1!A2", "Sheet1!A3"]);

    let err = wb
        .search(
            "[unclosed",
            SearchOptions {
                mode: SearchMode::Regex,
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidSearchQuery);
}

#[test]
fn search_matches_numbers_and_booleans_via_text() {
    let mut wb = search_fixture();
    let hits = wb.search("42", SearchOptions::default()).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].reference, "Sheet1!A4");
    assert_eq!(hits[0].value, CellValue::Number(42.0));

    let hits = wb.search("TRUE", SearchOptions::default()).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].reference, "Sheet1!A5");
}

#[test]
fn search_respects_sheet_and_range_scope_and_limit() {
    let mut wb = search_fixture();
    let hits = wb
        .search(
            "north",
            SearchOptions {
                sheet: Some("Sheet1".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().all(|h| h.sheet == "Sheet1"));

    let hits = wb
        .search(
            "north",
            SearchOptions {
                range: Some("Sheet1!A1:A2".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].reference, "Sheet1!A2");

    let hits = wb
        .search(
            "north",
            SearchOptions {
                max_results: Some(2),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(hits.len(), 2);
}

#[test]
fn search_include_hidden_controls_hidden_sheet_visibility() {
    let mut wb = search_fixture();
    wb.set_sheet_visibility("Inputs", SheetVisibility::Hidden)
        .unwrap();

    let default_hits = wb.search("north", SearchOptions::default()).unwrap();
    let default_sheets: Vec<_> = default_hits.iter().map(|m| m.sheet.as_str()).collect();
    assert!(default_sheets.contains(&"Inputs"));

    let visible_only = wb
        .search(
            "north",
            SearchOptions {
                include_hidden: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    let visible_sheets: Vec<_> = visible_only.iter().map(|m| m.sheet.as_str()).collect();
    assert!(!visible_sheets.contains(&"Inputs"));
    assert!(visible_sheets.contains(&"Sheet1"));

    let explicit_sheet = wb
        .search(
            "north",
            SearchOptions {
                sheet: Some("Inputs".to_string()),
                include_hidden: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(explicit_sheet.len(), 1);
    assert_eq!(explicit_sheet[0].sheet, "Inputs");
}

#[test]
fn search_diagnostics_empty_query_and_missing_sheet() {
    let mut wb = search_fixture();
    let err = wb.search("", SearchOptions::default()).unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidSearchQuery);

    let err = wb
        .search(
            "x",
            SearchOptions {
                sheet: Some("Ghost".to_string()),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::MissingSheet);
}
