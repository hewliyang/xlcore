//! Hyperlinks + cell comments. Both live next to the worksheet and need
//! a tiny relationship lookup, so they share one extractor module to
//! keep `lib.rs` clean.
//!
//! Hyperlinks: the worksheet's `<hyperlinks>` block holds `<hyperlink>`
//! children with a cell `ref`, an optional `r:id` (external rel id),
//! and optional `location` / `tooltip` / `display`. We resolve `r:id`
//! to an absolute URL via `WorksheetPart::get_hyperlink_relationship`.
//!
//! Comments: there's a `WorksheetCommentsPart` (`xl/comments<N>.xml`)
//! with `<authors>` + `<commentList>`. Each `<comment ref="A1"
//! authorId="0">` carries a rich-text body in `<text>` (same `<r>`
//! shape as SST entries).

use crate::schema::*;
use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::sdk::SdkPart;
use xlcore_io::{parse_a1, parse_range};

pub fn extract_hyperlinks(
    doc: &mut SpreadsheetDocument,
    ws_part: &WorksheetPart,
    ws: &ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main::Worksheet,
) -> Vec<Hyperlink> {
    let Some(hl_block) = ws.x_hyperlinks.as_ref() else { return Vec::new(); };
    let mut out = Vec::with_capacity(hl_block.x_hyperlink.len());
    for h in &hl_block.x_hyperlink {
        let r = h.reference.as_str();
        let range = if let Some(((r1, c1), (r2, c2))) = parse_range(r) {
            Merge { r1, c1, r2, c2 }
        } else if let Some((r1, c1)) = parse_a1(r) {
            Merge { r1, c1, r2: r1, c2: c1 }
        } else {
            continue;
        };
        // Resolve `r:id` to the rel target (an absolute URL for external
        // links). The SDK exposes hyperlink rels separately from the
        // internal-part rels charts use.
        let target = h.id.as_ref()
            .and_then(|rid| ws_part.get_hyperlink_relationship(doc, rid.as_str()))
            .map(|rel| rel.target().to_string());
        out.push(Hyperlink {
            range,
            target,
            location: h.location.as_ref().map(|s| s.as_str().to_string()),
            tooltip: h.tooltip.as_ref().map(|s| s.as_str().to_string()),
            display: h.display.as_ref().map(|s| s.as_str().to_string()),
        });
    }
    out
}

pub fn extract_comments(
    doc: &mut SpreadsheetDocument,
    ws_part: &WorksheetPart,
) -> Vec<Comment> {
    let Some(comments_part) = ws_part.worksheet_comments_part(doc) else { return Vec::new(); };
    let comments_part = comments_part.clone();
    let comments_root = match comments_part.root_element(doc) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let authors: Vec<String> = comments_root.authors.x_author.iter()
        .map(|a| a.xml_content.as_deref().unwrap_or("").to_string())
        .collect();
    let mut out = Vec::with_capacity(comments_root.comment_list.x_comment.len());
    for cmt in &comments_root.comment_list.x_comment {
        let Some((r, c)) = parse_a1(cmt.reference.as_str()) else { continue; };
        let author = authors.get(cmt.author_id as usize).cloned().unwrap_or_default();
        // CommentText is the same `<r>`-list shape as SST rich-text. Build
        // the flat string + per-run styling in lock-step.
        let mut text = String::new();
        let mut runs: Vec<TextRun> = Vec::new();
        if !cmt.comment_text.x_r.is_empty() {
            for run in &cmt.comment_text.x_r {
                let txt = run.text.xml_content.as_deref().unwrap_or("").to_string();
                text.push_str(&txt);
                runs.push(crate::text_run_from(run, txt));
            }
            // Collapse trivially-styled run lists; renderer falls back to
            // its default comment font in that case.
            if runs.iter().all(is_unstyled_run) {
                runs.clear();
            }
        } else if let Some(t) = cmt.comment_text.text.as_ref() {
            text = t.xml_content.as_deref().unwrap_or("").to_string();
        }
        out.push(Comment { r, c, author, text, runs });
    }
    out
}

fn is_unstyled_run(r: &TextRun) -> bool {
    !r.bold && !r.italic && !r.underline && !r.strike
        && r.size.is_none() && r.font_name.is_none() && r.color.is_none()
}
