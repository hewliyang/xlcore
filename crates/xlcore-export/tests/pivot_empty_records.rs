use base64::Engine;
use std::collections::BTreeMap;
use std::path::Path;

fn u32s(s: &str) -> Vec<u32> {
    let b = base64::engine::general_purpose::STANDARD.decode(s).unwrap();
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn i32s(s: &str) -> Vec<i32> {
    let b = base64::engine::general_purpose::STANDARD.decode(s).unwrap();
    b.chunks_exact(4)
        .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn pivot_grid(path: &str) -> BTreeMap<(u32, u32), String> {
    let layout = xlcore_export::extract(Path::new(path)).expect("extract");
    let sheet = layout
        .sheets
        .iter()
        .find(|s| s.name == "Pivot")
        .expect("pivot sheet");
    let rs = u32s(&sheet.cells.r);
    let cs = u32s(&sheet.cells.c);
    let vidx = i32s(&sheet.cells.value_idx);
    let mut out = BTreeMap::new();
    for i in 0..sheet.cells.count as usize {
        let vi = vidx[i];
        if vi < 0 {
            continue;
        }
        out.insert((rs[i], cs[i]), sheet.value_pool[vi as usize].clone());
    }
    out
}

#[test]
fn filter_arrows_carry_field_identity() {
    let root = env!("CARGO_MANIFEST_DIR");
    let path = format!("{root}/../../tests/fixtures/pivot/pivot-simple.xlsx");
    let layout = xlcore_export::extract(Path::new(&path)).expect("extract");
    let sheet = layout
        .sheets
        .iter()
        .find(|s| s.name == "Pivot")
        .expect("pivot sheet");
    let pivot = sheet.pivots.first().expect("pivot");
    let arrows = &pivot.filter_arrow_cells;
    assert!(!arrows.is_empty());
    let row_arrow = arrows
        .iter()
        .find(|a| a.axis == xlcore_export::PivotFilterAxis::Row)
        .expect("row arrow");
    assert_eq!(row_arrow.field, "Region");
}

#[test]
fn pivot_cells_do_not_duplicate_static_worksheet_cells() {
    let root = env!("CARGO_MANIFEST_DIR");
    let path = format!("{root}/../../tests/fixtures/pivot/pivot-simple.xlsx");
    let layout = xlcore_export::extract(Path::new(&path)).expect("extract");
    let sheet = layout
        .sheets
        .iter()
        .find(|s| s.name == "Pivot")
        .expect("pivot sheet");
    let pivot = sheet.pivots.first().expect("pivot");
    let (r1, c1, r2, c2) = (
        pivot.range.r1,
        pivot.range.c1,
        pivot.range.r2,
        pivot.range.c2,
    );
    let rs = u32s(&sheet.cells.r);
    let cs = u32s(&sheet.cells.c);
    let mut seen = std::collections::HashSet::new();
    for i in 0..sheet.cells.count as usize {
        let (r, c) = (rs[i], cs[i]);
        if r >= r1 && r <= r2 && c >= c1 && c <= c2 {
            assert!(
                seen.insert((r, c)),
                "duplicate cell at ({r},{c}) in pivot range"
            );
        }
    }
}

#[test]
fn synthesizes_records_from_worksheet_source_when_cache_empty() {
    let root = env!("CARGO_MANIFEST_DIR");
    let base = format!("{root}/../../tests/fixtures/pivot");
    let populated = pivot_grid(&format!("{base}/pivot-simple.xlsx"));
    let empty = pivot_grid(&format!("{base}/pivot-empty-records.xlsx"));

    assert!(!populated.is_empty());
    assert_eq!(populated, empty);
    assert!(empty.values().any(|v| v == "Grand Total"));
    assert!(empty.values().any(|v| v == "Region"));
}
