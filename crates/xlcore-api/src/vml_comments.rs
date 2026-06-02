use ooxmlsdk::parts::vml_drawing_part::VmlDrawingPart;
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::sdk::SdkPart;
use xlcore_io::spreadsheetml as x;

use crate::errors::sdk_err_to_api;
use crate::Result;

pub(crate) fn sync_vml_comment_indicators(
    doc: &mut xlcore_io::SpreadsheetDocument,
    ws_part: &WorksheetPart,
) -> Result<()> {
    let cells = collect_comment_cells(doc, ws_part)?;
    if cells.is_empty() {
        return Ok(());
    }

    let existing_vml = ws_part.vml_drawing_parts(doc).into_iter().next();
    let vml_part: VmlDrawingPart = match existing_vml {
        Some(part) => part,
        None => ws_part
            .add_new_part_auto_id::<_, VmlDrawingPart>(doc)
            .map_err(sdk_err_to_api)?,
    };
    let rid = vml_part
        .relationship_id()
        .map(|s| s.to_string())
        .unwrap_or_default();

    vml_part
        .set_data(doc, build_vml(&cells).into_bytes())
        .map_err(sdk_err_to_api)?;

    let ws_root = ws_part.root_element_mut(doc).map_err(sdk_err_to_api)?;
    if ws_root.legacy_drawing.is_none() {
        ws_root.legacy_drawing = Some(x::LegacyDrawing { id: rid });
    }
    Ok(())
}

fn collect_comment_cells(
    doc: &mut xlcore_io::SpreadsheetDocument,
    ws_part: &WorksheetPart,
) -> Result<Vec<(u32, u32)>> {
    let Some(comments_part) = ws_part.worksheet_comments_part(doc) else {
        return Ok(Vec::new());
    };
    let comments_part = comments_part.clone();
    let root = comments_part.root_element(doc).map_err(sdk_err_to_api)?;
    let mut out = Vec::with_capacity(root.comment_list.comment.len());
    for cmt in &root.comment_list.comment {
        if let Some((row, column)) = xlcore_io::parse_a1(cmt.reference.as_str()) {
            out.push((row, column));
        }
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

fn build_vml(cells: &[(u32, u32)]) -> String {
    let mut buf = String::new();
    buf.push_str(
        "<xml xmlns:v=\"urn:schemas-microsoft-com:vml\" \
xmlns:o=\"urn:schemas-microsoft-com:office:office\" \
xmlns:x=\"urn:schemas-microsoft-com:office:excel\">\
<o:shapelayout v:ext=\"edit\"><o:idmap v:ext=\"edit\" data=\"1\"/></o:shapelayout>\
<v:shapetype id=\"_x0000_t202\" coordsize=\"21600,21600\" o:spt=\"202\" \
path=\"m,l,21600r21600,l21600,xe\">\
<v:stroke joinstyle=\"miter\"/>\
<v:path gradientshapeok=\"t\" o:connecttype=\"rect\"/>\
</v:shapetype>",
    );
    for (idx, (row, column)) in cells.iter().enumerate() {
        let shape_id = 1025 + idx;
        let row0 = row.saturating_sub(1);
        let col0 = column.saturating_sub(1);
        let anchor_left = col0 + 1;
        let anchor_top = row0.saturating_sub(1);
        let anchor_right = col0 + 3;
        let anchor_bottom = row0 + 4;
        buf.push_str(&format!(
            "<v:shape id=\"_x0000_s{shape_id}\" type=\"#_x0000_t202\" \
style='position:absolute;margin-left:59.25pt;margin-top:1.5pt;width:108pt;\
height:59.25pt;z-index:{z};visibility:hidden' \
fillcolor=\"#ffffe1\" o:insetmode=\"auto\">\
<v:fill color2=\"#ffffe1\"/>\
<v:shadow on=\"t\" color=\"black\" obscured=\"t\"/>\
<v:path o:connecttype=\"none\"/>\
<v:textbox style='mso-direction-alt:auto'><div style='text-align:left'/></v:textbox>\
<x:ClientData ObjectType=\"Note\">\
<x:MoveWithCells/>\
<x:SizeWithCells/>\
<x:Anchor>{anchor_left}, 15, {anchor_top}, 10, {anchor_right}, 31, {anchor_bottom}, 1</x:Anchor>\
<x:AutoFill>False</x:AutoFill>\
<x:Row>{row0}</x:Row>\
<x:Column>{col0}</x:Column>\
</x:ClientData>\
</v:shape>",
            z = idx + 1,
        ));
    }
    buf.push_str("</xml>");
    buf
}
