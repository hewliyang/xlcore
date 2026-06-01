use ooxmlsdk::parts::workbook_part::WorkbookPart;
use ooxmlsdk::parts::workbook_person_part::WorkbookPersonPart;
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::parts::worksheet_threaded_comments_part::WorksheetThreadedCommentsPart;
use ooxmlsdk::schemas::schemas_microsoft_com_office_spreadsheetml_2018_threadedcomments as tc;
use xlcore_types::{ApiError, ApiErrorCode, ThreadedNoteInfo, ThreadedNotePatch};

use crate::errors::sdk_err_to_api;
use crate::refs::ranges_overlap;
use crate::{Result, Workbook};

impl Workbook {
    pub fn threaded_notes(&mut self, sheet: impl AsRef<str>) -> Result<Vec<ThreadedNoteInfo>> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let person_map = load_person_map(&mut self.doc)?;
        let mut out = Vec::new();
        let parts: Vec<_> = ws_part.worksheet_threaded_comments_parts(&self.doc).collect();
        for tc_part in parts {
            let root = tc_part
                .root_element(&mut self.doc)
                .map_err(sdk_err_to_api)?;
            for note in &root.threaded_comment {
                out.push(note_to_info(&sheet, note, &person_map));
            }
        }
        Ok(out)
    }

    pub fn add_threaded_note(
        &mut self,
        reference: impl AsRef<str>,
        patch: ThreadedNotePatch,
    ) -> Result<ThreadedNoteInfo> {
        let reference = reference.as_ref();
        if patch.text.is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidThreadedNote,
                "threaded note text is empty",
            )
            .with_ref(reference));
        }
        let cell_ref = self.resolve_cell_ref(reference)?;
        let cell_ref_str = format!(
            "{}{}",
            xlcore_io::col_label(cell_ref.column),
            cell_ref.row
        );
        let author = patch.author.clone().unwrap_or_else(|| "xlcore".to_string());
        let person_id = upsert_person(&mut self.doc, &author)?;
        let id = new_guid();
        let date = patch.date.clone().unwrap_or_else(now_iso8601);

        let ws_part = self.worksheet_part_for_sheet(&cell_ref.sheet)?;
        let tc_part = ensure_threaded_comments_part(&mut self.doc, &ws_part)?;
        let root = tc_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        root.threaded_comment.push(tc::ThreadedComment {
            r#ref: Some(cell_ref_str.clone().into()),
            d_t: Some(date.clone().into()),
            person_id: person_id.clone().into(),
            id: id.clone().into(),
            parent_id: None,
            done: None,
            threaded_comment_text: Some(patch.text.clone().into()),
            ..Default::default()
        });

        Ok(ThreadedNoteInfo {
            sheet: cell_ref.sheet,
            reference: cell_ref_str,
            row: cell_ref.row,
            column: cell_ref.column,
            id,
            parent_id: None,
            person_id,
            author,
            text: patch.text,
            date: Some(date),
            done: false,
        })
    }

    pub fn reply_threaded_note(
        &mut self,
        parent_id: impl AsRef<str>,
        patch: ThreadedNotePatch,
    ) -> Result<ThreadedNoteInfo> {
        let parent_id = parent_id.as_ref();
        if patch.text.is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidThreadedNote,
                "threaded note text is empty",
            ));
        }
        let (sheet, parent_ref, row, column) = self.find_root_note(parent_id)?;
        let author = patch.author.clone().unwrap_or_else(|| "xlcore".to_string());
        let person_id = upsert_person(&mut self.doc, &author)?;
        let id = new_guid();
        let date = patch.date.clone().unwrap_or_else(now_iso8601);

        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let tc_part = ensure_threaded_comments_part(&mut self.doc, &ws_part)?;
        let root = tc_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        root.threaded_comment.push(tc::ThreadedComment {
            r#ref: Some(parent_ref.clone().into()),
            d_t: Some(date.clone().into()),
            person_id: person_id.clone().into(),
            id: id.clone().into(),
            parent_id: Some(parent_id.to_string().into()),
            done: None,
            threaded_comment_text: Some(patch.text.clone().into()),
            ..Default::default()
        });

        Ok(ThreadedNoteInfo {
            sheet,
            reference: parent_ref,
            row,
            column,
            id,
            parent_id: Some(parent_id.to_string()),
            person_id,
            author,
            text: patch.text,
            date: Some(date),
            done: false,
        })
    }

    pub fn remove_threaded_thread(
        &mut self,
        reference: impl AsRef<str>,
    ) -> Result<Vec<ThreadedNoteInfo>> {
        let reference = reference.as_ref();
        let range_ref = self.resolve_range_ref(reference)?;
        let sheet = range_ref.sheet.clone();
        let person_map = load_person_map(&mut self.doc)?;
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let parts: Vec<_> = ws_part.worksheet_threaded_comments_parts(&self.doc).collect();
        let mut removed = Vec::new();
        for tc_part in parts {
            let root = tc_part
                .root_element_mut(&mut self.doc)
                .map_err(sdk_err_to_api)?;
            let mut kept = Vec::with_capacity(root.threaded_comment.len());
            for note in root.threaded_comment.drain(..) {
                let cell_ref = note.r#ref.as_ref().map(|s| s.as_str()).unwrap_or("");
                let hit = match xlcore_io::parse_a1(cell_ref) {
                    Some((row, column)) => ranges_overlap(
                        range_ref.start_row,
                        range_ref.start_column,
                        range_ref.end_row,
                        range_ref.end_column,
                        row,
                        column,
                        row,
                        column,
                    ),
                    None => false,
                };
                if hit {
                    removed.push(note_to_info(&sheet, &note, &person_map));
                } else {
                    kept.push(note);
                }
            }
            let empty = kept.is_empty();
            root.threaded_comment = kept;
            if empty {
                let _ = self.doc.delete_part(tc_part);
            }
        }
        Ok(removed)
    }

    fn find_root_note(&mut self, id: &str) -> Result<(String, String, u32, u32)> {
        let sheets: Vec<String> = self.sheets()?.iter().map(|s| s.name.clone()).collect();
        for sheet in sheets {
            let ws_part = self.worksheet_part_for_sheet(&sheet)?;
            let parts: Vec<_> = ws_part.worksheet_threaded_comments_parts(&self.doc).collect();
            for tc_part in parts {
                let root = tc_part
                    .root_element(&mut self.doc)
                    .map_err(sdk_err_to_api)?;
                if let Some(note) = root.threaded_comment.iter().find(|n| n.id.as_str() == id) {
                    let cell_ref = note
                        .r#ref
                        .as_ref()
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_default();
                    let (row, column) =
                        xlcore_io::parse_a1(&cell_ref).unwrap_or((0, 0));
                    return Ok((sheet.clone(), cell_ref, row, column));
                }
            }
        }
        Err(ApiError::new(
            ApiErrorCode::InvalidThreadedNote,
            format!("threaded note id {id} not found"),
        ))
    }
}

fn note_to_info(
    sheet: &str,
    note: &tc::ThreadedComment,
    person_map: &std::collections::HashMap<String, String>,
) -> ThreadedNoteInfo {
    let cell_ref = note
        .r#ref
        .as_ref()
        .map(|s| s.as_str().to_string())
        .unwrap_or_default();
    let (row, column) = xlcore_io::parse_a1(&cell_ref).unwrap_or((0, 0));
    let person_id = note.person_id.as_str().to_string();
    let author = person_map.get(&person_id).cloned().unwrap_or_default();
    let text = note
        .threaded_comment_text
        .as_ref()
        .map(|s| s.as_str().to_string())
        .unwrap_or_default();
    let date = note.d_t.as_ref().map(|s| s.as_str().to_string());
    let done = note
        .done
        .as_ref()
        .map(|b| bool::from(b.clone()))
        .unwrap_or(false);
    ThreadedNoteInfo {
        sheet: sheet.to_string(),
        reference: cell_ref,
        row,
        column,
        id: note.id.as_str().to_string(),
        parent_id: note.parent_id.as_ref().map(|s| s.as_str().to_string()),
        person_id,
        author,
        text,
        date,
        done,
    }
}

fn load_person_map(
    doc: &mut xlcore_io::SpreadsheetDocument,
) -> Result<std::collections::HashMap<String, String>> {
    let wb_part = doc.workbook_part().map_err(sdk_err_to_api)?.clone();
    let mut map = std::collections::HashMap::new();
    let parts: Vec<_> = wb_part.workbook_person_parts(doc).collect();
    for part in parts {
        let root = part.root_element(doc).map_err(sdk_err_to_api)?;
        for person in &root.person {
            map.insert(
                person.id.as_str().to_string(),
                person.display_name.as_str().to_string(),
            );
        }
    }
    Ok(map)
}

fn upsert_person(doc: &mut xlcore_io::SpreadsheetDocument, display_name: &str) -> Result<String> {
    let wb_part = doc.workbook_part().map_err(sdk_err_to_api)?.clone();
    let existing: Vec<_> = wb_part.workbook_person_parts(doc).collect();
    for part in &existing {
        let root = part.root_element(doc).map_err(sdk_err_to_api)?;
        if let Some(person) = root
            .person
            .iter()
            .find(|p| p.display_name.as_str() == display_name)
        {
            return Ok(person.id.as_str().to_string());
        }
    }
    let person_part = match existing.into_iter().next() {
        Some(part) => part,
        None => create_person_part(doc, &wb_part)?,
    };
    let id = new_guid();
    let root = person_part
        .root_element_mut(doc)
        .map_err(sdk_err_to_api)?;
    root.person.push(tc::Person {
        display_name: display_name.to_string().into(),
        id: id.clone().into(),
        user_id: Some(format!("S::{display_name}::{id}").into()),
        provider_id: Some("None".to_string().into()),
        ..Default::default()
    });
    Ok(id)
}

fn create_person_part(
    doc: &mut xlcore_io::SpreadsheetDocument,
    wb_part: &WorkbookPart,
) -> Result<WorkbookPersonPart> {
    let part: WorkbookPersonPart = wb_part
        .add_new_part_auto_id(doc)
        .map_err(sdk_err_to_api)?;
    part.set_root_element(doc, default_person_list())
        .map_err(sdk_err_to_api)?;
    Ok(part)
}

fn default_person_list() -> tc::PersonList {
    tc::PersonList {
        xmlns: vec![ooxmlsdk::common::XmlNamespaceDecl::new(
            "xltc",
            "http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments",
        )],
        ..Default::default()
    }
}

fn default_threaded_comments_root() -> tc::ThreadedComments {
    tc::ThreadedComments {
        xmlns: vec![ooxmlsdk::common::XmlNamespaceDecl::new(
            "xltc",
            "http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments",
        )],
        ..Default::default()
    }
}

fn ensure_threaded_comments_part(
    doc: &mut xlcore_io::SpreadsheetDocument,
    ws_part: &WorksheetPart,
) -> Result<WorksheetThreadedCommentsPart> {
    if let Some(existing) = ws_part.worksheet_threaded_comments_parts(doc).next() {
        return Ok(existing.clone());
    }
    let part: WorksheetThreadedCommentsPart = ws_part
        .add_new_part_auto_id(doc)
        .map_err(sdk_err_to_api)?;
    part.set_root_element(doc, default_threaded_comments_root())
        .map_err(sdk_err_to_api)?;
    Ok(part)
}

fn new_guid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id() as u64;
    let a = nanos as u32;
    let b = ((nanos >> 32) & 0xFFFF) as u16;
    let c = (((counter & 0x0FFF) | 0x4000) & 0xFFFF) as u16;
    let d = ((pid & 0x3FFF) | 0x8000) as u16;
    let e = mix(nanos.wrapping_add(counter).wrapping_add(pid));
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:04X}-{:012X}}}",
        a,
        b,
        c,
        d,
        e & 0x0000_FFFF_FFFF_FFFF
    )
}

fn mix(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^= x >> 33;
    x
}

fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, mo, d, h, mi, s) = epoch_to_ymdhms(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.00", y, mo, d, h, mi, s)
}

fn epoch_to_ymdhms(secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400) as u32;
    let h = tod / 3600;
    let mi = (tod % 3600) / 60;
    let s = tod % 60;
    let (y, mo, d) = days_to_ymd(days);
    (y, mo, d, h, mi, s)
}

fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    let mut z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    z = 0;
    let _ = z;
    (y as i32, m, d)
}
