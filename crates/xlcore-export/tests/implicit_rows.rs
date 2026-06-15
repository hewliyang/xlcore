use base64::Engine;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(rel)
}

fn decode_u32(b64: &str) -> Vec<u32> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .expect("valid base64");
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

#[test]
fn implicit_row_indices_are_recovered() {
    let layout = xlcore_export::extract(fixture(
        "producer-quirks/spreadjs-implicit-row-index.xlsx",
    ))
    .expect("fixture should extract");

    let sheet = &layout.sheets[0];

    assert_eq!(
        sheet.cells.count, 5,
        "all five cells must survive; before the fix the three `r`-less rows were dropped (only rows 1 and 5 remained, count == 2)"
    );
    assert_eq!(
        decode_u32(&sheet.cells.r),
        vec![1, 2, 3, 5, 6],
        "row coordinates must include the implicit rows 2, 3 and 6"
    );
    assert_eq!(
        decode_u32(&sheet.row_meta.index),
        vec![1, 2, 3, 5, 6],
        "row metadata must be emitted for the implicit rows too"
    );
}
