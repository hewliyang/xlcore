use ooxmlsdk::parts::worksheet_comments_part::WorksheetCommentsPart;
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::sdk::SdkPart;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{ApiError, ApiErrorCode, CommentInfo, CommentPatch};

use crate::errors::sdk_err_to_api;
use crate::refs::{qualify_ref, ranges_overlap};
use crate::vml_comments::sync_vml_comment_indicators;
use crate::{Result, Workbook};

impl Workbook {
    pub fn comments(&mut self, sheet: impl AsRef<str>) -> Result<Vec<CommentInfo>> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let Some(comments_part) = ws_part.worksheet_comments_part(&self.doc) else {
            return Ok(Vec::new());
        };
        let root = comments_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let authors: Vec<String> = root
            .authors
            .author
            .iter()
            .map(|a| a.xml_content.as_deref().unwrap_or("").to_string())
            .collect();
        let mut out = Vec::with_capacity(root.comment_list.comment.len());
        for cmt in &root.comment_list.comment {
            let Some((row, column)) = xlcore_io::parse_a1(cmt.reference.as_str()) else {
                continue;
            };
            let author = authors
                .get(cmt.author_id as usize)
                .cloned()
                .unwrap_or_default();
            if is_threaded_shadow_author(&author) {
                continue;
            }
            let text = comment_plain_text(cmt);
            out.push(CommentInfo {
                sheet: sheet.clone(),
                reference: format!("{}{}", xlcore_io::col_label(column), row),
                row,
                column,
                author,
                text,
            });
        }
        Ok(out)
    }

    pub fn set_comment(
        &mut self,
        sheet: impl AsRef<str>,
        reference: impl AsRef<str>,
        patch: CommentPatch,
    ) -> Result<CommentInfo> {
        let reference = qualify_ref(sheet.as_ref(), reference.as_ref())?;
        let reference = reference.as_str();
        let cell_ref = self.resolve_cell_ref(reference)?;
        if patch.text.is_empty() {
            return Err(
                ApiError::new(ApiErrorCode::InvalidComment, "comment text is empty")
                    .with_ref(reference)
                    .with_sheet(&cell_ref.sheet),
            );
        }
        let author = patch.author.clone().unwrap_or_else(|| "xlcore".to_string());
        let cell_ref_str = format!("{}{}", xlcore_io::col_label(cell_ref.column), cell_ref.row,);
        let ws_part = self.worksheet_part_for_sheet(&cell_ref.sheet)?;
        let comments_part = ensure_comments_part(&mut self.doc, &ws_part)?;
        let root = comments_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let author_id = upsert_author(root, &author);
        let shadow_author_ids: Vec<u32> = root
            .authors
            .author
            .iter()
            .enumerate()
            .filter(|(_, a)| is_threaded_shadow_author(a.xml_content.as_deref().unwrap_or("")))
            .map(|(i, _)| i as u32)
            .collect();
        root.comment_list.comment.retain(|cmt| {
            cmt.reference.as_str() != cell_ref_str.as_str()
                || shadow_author_ids.contains(&cmt.author_id)
        });
        root.comment_list.comment.push(x::Comment {
            reference: cell_ref_str.clone().into(),
            author_id: (author_id as u32).into(),
            comment_text: Box::new(x::CommentText {
                text: Some(x::Text(x::XstringType {
                    space: Some(ooxmlsdk::schemas::xml::SpaceProcessingModeValues::Preserve),
                    xml_content: Some(patch.text.clone().into()),
                    ..Default::default()
                })),
                ..Default::default()
            }),
            ..Default::default()
        });
        let sync_ws_part = self.worksheet_part_for_sheet(&cell_ref.sheet)?;
        sync_vml_comment_indicators(&mut self.doc, &sync_ws_part)?;
        Ok(CommentInfo {
            sheet: cell_ref.sheet,
            reference: cell_ref_str,
            row: cell_ref.row,
            column: cell_ref.column,
            author,
            text: patch.text,
        })
    }

    pub fn remove_comment(
        &mut self,
        sheet: impl AsRef<str>,
        reference: impl AsRef<str>,
    ) -> Result<Vec<CommentInfo>> {
        let reference = qualify_ref(sheet.as_ref(), reference.as_ref())?;
        let range_ref = self.resolve_range_ref(&reference)?;
        let sheet = range_ref.sheet.clone();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let Some(comments_part) = ws_part.worksheet_comments_part(&self.doc) else {
            return Ok(Vec::new());
        };
        let comments_part = comments_part.clone();
        let root = comments_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let authors: Vec<String> = root
            .authors
            .author
            .iter()
            .map(|a| a.xml_content.as_deref().unwrap_or("").to_string())
            .collect();
        let mut removed = Vec::new();
        let mut kept = Vec::with_capacity(root.comment_list.comment.len());
        for cmt in root.comment_list.comment.drain(..) {
            let author_str = authors
                .get(cmt.author_id as usize)
                .cloned()
                .unwrap_or_default();
            let is_shadow = is_threaded_shadow_author(&author_str);
            let hit = !is_shadow
                && match xlcore_io::parse_a1(cmt.reference.as_str()) {
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
                let (row, column) = xlcore_io::parse_a1(cmt.reference.as_str()).unwrap();
                let author = authors
                    .get(cmt.author_id as usize)
                    .cloned()
                    .unwrap_or_default();
                removed.push(CommentInfo {
                    sheet: sheet.clone(),
                    reference: cmt.reference.as_str().to_string(),
                    row,
                    column,
                    author,
                    text: comment_plain_text(&cmt),
                });
            } else {
                kept.push(cmt);
            }
        }
        root.comment_list.comment = kept;
        let part_empty = root.comment_list.comment.is_empty();
        if part_empty {
            if let Some(rid) = comments_part.relationship_id().map(|s| s.to_string()) {
                let _ = ws_part
                    .delete_part_by_id(&mut self.doc, rid.as_str())
                    .map_err(sdk_err_to_api);
            }
        } else {
            sync_vml_comment_indicators(&mut self.doc, &ws_part)?;
        }
        Ok(removed)
    }
}

pub(crate) fn comment_plain_text(cmt: &x::Comment) -> String {
    if !cmt.comment_text.run.is_empty() {
        let mut buf = String::new();
        for run in &cmt.comment_text.run {
            if let Some(t) = run.text.xml_content.as_deref() {
                buf.push_str(t);
            }
        }
        buf
    } else if let Some(t) = cmt.comment_text.text.as_ref() {
        t.xml_content.as_deref().unwrap_or("").to_string()
    } else {
        String::new()
    }
}

pub(crate) fn upsert_author(root: &mut x::Comments, author: &str) -> usize {
    if let Some(idx) = root
        .authors
        .author
        .iter()
        .position(|a| a.xml_content.as_deref().unwrap_or("") == author)
    {
        return idx;
    }
    let idx = root.authors.author.len();
    root.authors.author.push(x::Author(x::XstringType {
        xml_content: Some(author.to_string().into()),
        ..Default::default()
    }));
    idx
}

pub(crate) fn is_threaded_shadow_author(author: &str) -> bool {
    author.starts_with("tc=")
}

fn default_comments_root() -> x::Comments {
    x::Comments {
        xmlns: crate::ooxml_header::spreadsheetml_default_only(),
        xml_header: crate::ooxml_header::STANDALONE,
        ..Default::default()
    }
}

pub(crate) fn ensure_comments_part(
    doc: &mut xlcore_io::SpreadsheetDocument,
    ws_part: &WorksheetPart,
) -> Result<WorksheetCommentsPart> {
    if let Some(existing) = ws_part.worksheet_comments_part(doc) {
        return Ok(existing.clone());
    }
    let part: WorksheetCommentsPart = ws_part
        .add_new_part_auto_id::<_, WorksheetCommentsPart>(doc)
        .map_err(sdk_err_to_api)?;
    part.set_root_element(doc, default_comments_root())
        .map_err(sdk_err_to_api)?;
    Ok(part)
}
