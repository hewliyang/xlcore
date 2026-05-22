use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(rel)
}

#[test]
fn loads_spreadjs_textrotation_nan_with_report() {
    let path = fixture("producer-quirks/spreadjs-textrotation-nan.xlsx");
    let bytes = std::fs::read(&path).expect("read fixture");

    let (_doc, report) = xlcore_io::open_bytes_with_report(bytes)
        .expect("SpreadJS file should load via the precompile pipeline");

    let fixed = report
        .fixes
        .iter()
        .find(|d| d.field.as_deref() == Some("textRotation") && d.value.as_deref() == Some("NaN"))
        .expect("textRotation=NaN fix should be recorded");
    assert!(
        fixed.occurrences >= 1,
        "should have stripped at least one occurrence"
    );
    assert!(
        matches!(fixed.kind, xlcore_io::SchemaErrorKind::InvalidFieldValue),
        "kind should classify as InvalidFieldValue"
    );

    assert_ne!(
        fixed.part, "*",
        "part should be a real zip path, got: {:?}",
        fixed.part
    );
    assert!(
        fixed.part.ends_with(".xml"),
        "part should be an xml entry, got: {:?}",
        fixed.part
    );
}

#[test]
fn random_bytes_yield_zip_error() {
    let bytes = b"not a real xlsx, just some text".to_vec();
    let err = xlcore_io::open_bytes_with_report(bytes).unwrap_err();
    assert_eq!(err.code(), "Zip", "got: {err:?}");
}

#[test]
fn pdf_yields_zip_error() {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    bytes.extend_from_slice(&[0u8; 64]);
    let err = xlcore_io::open_bytes_with_report(bytes).unwrap_err();
    assert_eq!(err.code(), "Zip", "got: {err:?}");
}

#[test]
fn ole_compound_file_yields_zip_error() {
    let mut bytes = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    bytes.extend_from_slice(&[0u8; 512]);
    let err = xlcore_io::open_bytes_with_report(bytes).unwrap_err();
    assert_eq!(err.code(), "Zip", "got: {err:?}");
}
