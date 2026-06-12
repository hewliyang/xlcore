use crate::*;

#[test]
fn comments_add_list_update_remove_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "Units").unwrap();
    wb.set_comment("Sheet1", "A1",
        CommentPatch {
            text: "Units sold this quarter".to_string(),
            author: Some("Mario".to_string()),
        },
    )
    .unwrap();
    wb.set_comment("Sheet1", "B2",
        CommentPatch {
            text: "double check".to_string(),
            author: None,
        },
    )
    .unwrap();

    let list = wb.comments("Sheet1").unwrap();
    assert_eq!(list.len(), 2);
    let a1 = list.iter().find(|c| c.reference == "A1").unwrap();
    assert_eq!(a1.author, "Mario");
    assert_eq!(a1.text, "Units sold this quarter");

    let updated = wb
        .set_comment("Sheet1", "A1",
            CommentPatch {
                text: "updated".to_string(),
                author: Some("Mario".to_string()),
            },
        )
        .unwrap();
    assert_eq!(updated.text, "updated");
    assert_eq!(wb.comments("Sheet1").unwrap().len(), 2);

    let empty = wb.set_comment("Sheet1", "C3", CommentPatch::default());
    assert_eq!(empty.unwrap_err().code, ApiErrorCode::InvalidComment);

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.comments("Sheet1").unwrap();
    assert_eq!(after.len(), 2);
    assert!(after
        .iter()
        .any(|c| c.reference == "A1" && c.text == "updated"));

    let removed = reopened.remove_comment("Sheet1", "A1:B2").unwrap();
    assert_eq!(removed.len(), 2);
    assert!(reopened.comments("Sheet1").unwrap().is_empty());

    let bytes = reopened.save_bytes().unwrap();
    let mut reopened2 = Workbook::open_bytes(bytes).unwrap();
    assert!(reopened2.comments("Sheet1").unwrap().is_empty());
}

#[test]
fn threaded_notes_add_reply_list_remove_and_round_trip() {
    use crate::ThreadedNotePatch;

    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "Units").unwrap();
    let root = wb
        .add_threaded_note("Sheet1", "A1",
            ThreadedNotePatch {
                text: "check this".to_string(),
                author: Some("Mario".to_string()),
                date: None,
            },
        )
        .unwrap();
    assert_eq!(root.reference, "A1");
    assert_eq!(root.author, "Mario");
    assert!(root.parent_id.is_none());

    let reply = wb
        .reply_threaded_note(
            &root.id,
            ThreadedNotePatch {
                text: "on it".to_string(),
                author: Some("Luigi".to_string()),
                date: None,
            },
        )
        .unwrap();
    assert_eq!(reply.reference, "A1");
    assert_eq!(reply.parent_id.as_deref(), Some(root.id.as_str()));
    assert_ne!(reply.person_id, root.person_id);

    let empty = wb.add_threaded_note("Sheet1", "B2", ThreadedNotePatch::default());
    assert_eq!(empty.unwrap_err().code, ApiErrorCode::InvalidThreadedNote);

    let list = wb.threaded_notes("Sheet1").unwrap();
    assert_eq!(list.len(), 2);
    assert!(list
        .iter()
        .any(|n| n.text == "check this" && n.author == "Mario"));
    assert!(list
        .iter()
        .any(|n| n.text == "on it" && n.author == "Luigi"));
    assert!(wb.comments("Sheet1").unwrap().is_empty());

    let bytes = wb.save_bytes().unwrap();
    {
        let cursor = std::io::Cursor::new(&bytes);
        let mut zip = zip::ZipArchive::new(cursor).unwrap();
        let names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(
            names
                .iter()
                .any(|n| n.starts_with("xl/comments") && n.ends_with(".xml")),
            "classic shadow comments part missing: {names:?}"
        );
        let mut buf = String::new();
        use std::io::Read;
        zip.by_name(names.iter().find(|n| n.starts_with("xl/comments")).unwrap())
            .unwrap()
            .read_to_string(&mut buf)
            .unwrap();
        assert!(
            buf.contains("tc="),
            "classic shadow author tc= missing in {buf}"
        );
        assert!(buf.contains("check this"));
        assert!(
            !buf.contains("on it"),
            "replies must not produce a second legacy comment per cell"
        );
    }

    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.threaded_notes("Sheet1").unwrap();
    assert_eq!(after.len(), 2);
    assert!(after
        .iter()
        .any(|n| n.author == "Luigi" && n.parent_id.is_some()));
    assert!(reopened.comments("Sheet1").unwrap().is_empty());

    let removed = reopened.remove_threaded_thread("Sheet1", "A1").unwrap();
    assert_eq!(removed.len(), 2);
    assert!(reopened.threaded_notes("Sheet1").unwrap().is_empty());

    let bytes = reopened.save_bytes().unwrap();
    {
        let cursor = std::io::Cursor::new(&bytes);
        let mut zip = zip::ZipArchive::new(cursor).unwrap();
        let names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(
            !names
                .iter()
                .any(|n| n.starts_with("xl/comments") && n.ends_with(".xml")),
            "classic shadow comments part should be gone: {names:?}"
        );
    }
    let mut reopened2 = Workbook::open_bytes(bytes).unwrap();
    assert!(reopened2.threaded_notes("Sheet1").unwrap().is_empty());
}

#[test]
fn threaded_note_shadow_coexists_with_classic_comment() {
    use crate::ThreadedNotePatch;

    let mut wb = Workbook::new().unwrap();
    wb.set_comment("Sheet1", "B2",
        CommentPatch {
            text: "old school".into(),
            author: Some("Peach".into()),
        },
    )
    .unwrap();
    wb.add_threaded_note("Sheet1", "A1",
        ThreadedNotePatch {
            text: "modern".into(),
            author: Some("Mario".into()),
            date: None,
        },
    )
    .unwrap();

    let classics = wb.comments("Sheet1").unwrap();
    assert_eq!(classics.len(), 1);
    assert_eq!(classics[0].reference, "B2");
    assert_eq!(classics[0].author, "Peach");

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let classics = reopened.comments("Sheet1").unwrap();
    assert_eq!(classics.len(), 1);
    assert_eq!(reopened.threaded_notes("Sheet1").unwrap().len(), 1);

    reopened
        .set_comment("Sheet1", "B2",
            CommentPatch {
                text: "old school v2".into(),
                author: Some("Peach".into()),
            },
        )
        .unwrap();
    assert_eq!(reopened.threaded_notes("Sheet1").unwrap().len(), 1);
    let classics = reopened.comments("Sheet1").unwrap();
    assert_eq!(classics.len(), 1);
    assert_eq!(classics[0].text, "old school v2");

    reopened.remove_comment("Sheet1", "B2").unwrap();
    assert!(reopened.comments("Sheet1").unwrap().is_empty());
    assert_eq!(reopened.threaded_notes("Sheet1").unwrap().len(), 1);
}

#[test]
fn comment_emits_vml_legacy_drawing_indicator() {
    let mut wb = Workbook::new().unwrap();
    wb.set_comment("Sheet1", "B3",
        CommentPatch {
            text: "note".into(),
            author: Some("Mario".into()),
        },
    )
    .unwrap();
    let bytes = wb.save_bytes().unwrap();

    let cursor = std::io::Cursor::new(&bytes);
    let mut zip = zip::ZipArchive::new(cursor).unwrap();
    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect();
    let vml_name = names
        .iter()
        .find(|n| n.ends_with(".vml"))
        .unwrap_or_else(|| panic!("vml drawing missing: {names:?}"))
        .clone();
    let mut buf = String::new();
    use std::io::Read;
    zip.by_name(&vml_name)
        .unwrap()
        .read_to_string(&mut buf)
        .unwrap();
    assert!(
        buf.contains("x:ClientData ObjectType=\"Note\""),
        "vml missing client data: {buf}"
    );
    assert!(buf.contains("<x:Row>2</x:Row>"), "vml missing row: {buf}");
    assert!(
        buf.contains("<x:Column>1</x:Column>"),
        "vml missing column: {buf}"
    );

    let sheet_name = names
        .iter()
        .find(|n| n.starts_with("xl/worksheets/sheet") && n.ends_with(".xml"))
        .unwrap()
        .clone();
    let mut sheet_buf = String::new();
    zip.by_name(&sheet_name)
        .unwrap()
        .read_to_string(&mut sheet_buf)
        .unwrap();
    assert!(
        sheet_buf.contains("legacyDrawing"),
        "sheet missing legacyDrawing: {sheet_buf}"
    );

    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    assert_eq!(reopened.comments("Sheet1").unwrap().len(), 1);
}
