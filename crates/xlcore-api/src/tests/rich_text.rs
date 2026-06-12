use crate::*;

fn run(text: &str, font: Option<FontPatch>) -> RichTextRun {
    RichTextRun {
        text: text.to_string(),
        font,
    }
}

#[test]
fn rich_text_round_trips_runs_and_formatting() {
    let mut workbook = Workbook::new().unwrap();
    let red_bold = FontPatch {
        bold: Some(true),
        color: Some("#FF0000".into()),
        ..Default::default()
    };
    let info = workbook
        .set_rich_text_in(
            "Sheet1",
            "A1",
            vec![
                run("Hello ", None),
                run("world", Some(red_bold)),
                run(
                    "!",
                    Some(FontPatch {
                        italic: Some(true),
                        vert_align: Some(VertAlign::Superscript),
                        ..Default::default()
                    }),
                ),
            ],
        )
        .unwrap();

    assert_eq!(info.value, CellValue::String("Hello world!".to_string()));
    let rich = info.rich_text.clone().expect("rich_text present");
    assert_eq!(rich.runs.len(), 3);
    assert_eq!(rich.runs[0].text, "Hello ");
    assert!(rich.runs[0].font.is_none());

    let second = rich.runs[1].font.as_ref().unwrap();
    assert_eq!(second.bold, Some(true));
    assert_eq!(second.color.as_deref(), Some("#FFFF0000"));

    let third = rich.runs[2].font.as_ref().unwrap();
    assert_eq!(third.italic, Some(true));
    assert_eq!(third.vert_align, Some(VertAlign::Superscript));

    let reread = workbook.get_cell_in("Sheet1", "A1").unwrap();
    assert_eq!(reread.rich_text, info.rich_text);
}

#[test]
fn rich_text_survives_save_reload() {
    let mut workbook = Workbook::new().unwrap();
    workbook
        .set_rich_text_in(
            "Sheet1",
            "B2",
            vec![
                run("plain", None),
                run(
                    "styled",
                    Some(FontPatch {
                        bold: Some(true),
                        size: Some(18.0),
                        name: Some("Arial".into()),
                        ..Default::default()
                    }),
                ),
            ],
        )
        .unwrap();
    let bytes = workbook.save_bytes().unwrap();

    let mut reloaded = Workbook::open_bytes(bytes).unwrap();
    let info = reloaded.get_cell_in("Sheet1", "B2").unwrap();
    assert_eq!(info.value, CellValue::String("plainstyled".to_string()));
    let rich = info.rich_text.expect("rich_text present after reload");
    assert_eq!(rich.runs.len(), 2);
    let styled = rich.runs[1].font.as_ref().unwrap();
    assert_eq!(styled.bold, Some(true));
    assert_eq!(styled.size, Some(18.0));
    assert_eq!(styled.name.as_deref(), Some("Arial"));
}

#[test]
fn set_value_after_rich_text_clears_runs() {
    let mut workbook = Workbook::new().unwrap();
    workbook
        .set_rich_text_in("Sheet1", "A1", vec![run("a", None), run("b", None)])
        .unwrap();
    let info = workbook.set_value_in("Sheet1", "A1", "plain").unwrap();
    assert!(info.rich_text.is_none());
    assert_eq!(info.value, CellValue::String("plain".to_string()));
}

#[test]
fn empty_rich_text_runs_is_rejected() {
    let mut workbook = Workbook::new().unwrap();
    let err = workbook
        .set_rich_text_in("Sheet1", "A1", vec![])
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::Other);
}
