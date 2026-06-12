use crate::*;

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

#[test]
fn images_create_list_remove_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    let info = wb
        .set_image(ImagePatch {
            sheet: "Sheet1".to_string(),
            name: Some("Logo".to_string()),
            anchor: AnchorSpec::Cells(ChartAnchor {
                from_column: 1,
                from_row: 1,
                to_column: 5,
                to_row: 10,
                ..Default::default()
            }),
            bytes: PNG_1X1.to_vec(),
            format: None,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(info.sheet, "Sheet1");
    assert_eq!(info.name, "Logo");
    assert_eq!(info.format, ImageFormat::Png);
    assert_eq!(info.byte_len as usize, PNG_1X1.len());
    assert!(!info.id.is_empty());

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let images = reopened.images(None).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].format, ImageFormat::Png);
    assert_eq!(images[0].name, "Logo");
    assert_eq!(images[0].anchor.from_column, 1);
    assert_eq!(images[0].anchor.to_row, 10);
    assert_eq!(images[0].byte_len as usize, PNG_1X1.len());

    let id = images[0].id.clone();
    let removed = reopened.remove_image("Sheet1", &id).unwrap().unwrap();
    assert_eq!(removed.id, id);
    assert!(reopened.images(None).unwrap().is_empty());

    let bytes2 = reopened.save_bytes().unwrap();
    let mut reopened2 = Workbook::open_bytes(bytes2).unwrap();
    assert!(reopened2.images(None).unwrap().is_empty());
}

#[test]
fn images_rotation_crop_flip_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    let info = wb
        .set_image(ImagePatch {
            sheet: "Sheet1".to_string(),
            name: Some("Rotated".to_string()),
            anchor: AnchorSpec::Cells(ChartAnchor {
                from_column: 0,
                from_row: 0,
                to_column: 4,
                to_row: 8,
                ..Default::default()
            }),
            bytes: PNG_1X1.to_vec(),
            format: None,
            rotation_degrees: Some(90.0),
            crop_left_pct: Some(10.0),
            crop_top_pct: Some(20.0),
            crop_right_pct: Some(5.0),
            crop_bottom_pct: Some(15.0),
            flip_horizontal: Some(true),
            flip_vertical: Some(false),
        })
        .unwrap();
    assert_eq!(info.rotation_degrees, 90.0);
    assert_eq!(info.crop_left_pct, 10.0);
    assert!(info.flip_horizontal);
    assert!(!info.flip_vertical);

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let images = reopened.images(None).unwrap();
    assert_eq!(images.len(), 1);
    let got = &images[0];
    assert!((got.rotation_degrees - 90.0).abs() < 1e-3);
    assert!((got.crop_left_pct - 10.0).abs() < 1e-3);
    assert!((got.crop_top_pct - 20.0).abs() < 1e-3);
    assert!((got.crop_right_pct - 5.0).abs() < 1e-3);
    assert!((got.crop_bottom_pct - 15.0).abs() < 1e-3);
    assert!(got.flip_horizontal);
    assert!(!got.flip_vertical);
}

#[test]
fn images_rejects_non_finite_rotation_and_crop() {
    let mut wb = Workbook::new().unwrap();
    let err = wb
        .set_image(ImagePatch {
            sheet: "Sheet1".to_string(),
            name: None,
            anchor: AnchorSpec::Cells(ChartAnchor::default()),
            bytes: PNG_1X1.to_vec(),
            format: None,
            rotation_degrees: Some(f64::NAN),
            crop_left_pct: None,
            crop_top_pct: None,
            crop_right_pct: None,
            crop_bottom_pct: None,
            flip_horizontal: None,
            flip_vertical: None,
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidImage);

    let err = wb
        .set_image(ImagePatch {
            sheet: "Sheet1".to_string(),
            name: None,
            anchor: AnchorSpec::Cells(ChartAnchor::default()),
            bytes: PNG_1X1.to_vec(),
            format: None,
            rotation_degrees: None,
            crop_left_pct: Some(f64::INFINITY),
            crop_top_pct: None,
            crop_right_pct: None,
            crop_bottom_pct: None,
            flip_horizontal: None,
            flip_vertical: None,
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidImage);
}

#[test]
fn images_rejects_empty_bytes_and_unknown_format() {
    let mut wb = Workbook::new().unwrap();
    let err = wb
        .set_image(ImagePatch {
            sheet: "Sheet1".to_string(),
            name: None,
            anchor: AnchorSpec::Cells(ChartAnchor::default()),
            bytes: Vec::new(),
            format: None,
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidImage);

    let err = wb
        .set_image(ImagePatch {
            sheet: "Sheet1".to_string(),
            name: None,
            anchor: AnchorSpec::Cells(ChartAnchor::default()),
            bytes: b"not an image".to_vec(),
            format: None,
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidImage);
}
