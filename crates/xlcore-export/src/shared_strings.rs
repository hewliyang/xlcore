use crate::*;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as xspread;

/// rich-text runs so the renderer can preserve per-run bold/italic/color.
pub(crate) fn preload(
    doc: &mut xlcore_io::SpreadsheetDocument,
) -> (Vec<String>, Vec<Vec<TextRun>>) {
    let wb_part = match doc.workbook_part() {
        Ok(p) => p,
        Err(_) => return (Vec::new(), Vec::new()),
    };
    let sst_part = match wb_part.shared_string_table_part(doc) {
        Some(p) => p.clone(),
        None => return (Vec::new(), Vec::new()),
    };
    let sst = match sst_part.root_element(doc) {
        Ok(s) => s,
        Err(_) => return (Vec::new(), Vec::new()),
    };
    let mut texts = Vec::with_capacity(sst.x_si.len());
    let mut runs = Vec::with_capacity(sst.x_si.len());
    for item in &sst.x_si {
        // Plain `<t>` form -> no runs.
        if let Some(t) = &item.text {
            texts.push(t.xml_content.as_deref().unwrap_or("").to_string());
            runs.push(Vec::new());
            continue;
        }
        // `<r>` form -> build flat string + parallel TextRun list.
        let mut s = String::new();
        let mut rs: Vec<TextRun> = Vec::with_capacity(item.x_r.len());
        for r in &item.x_r {
            let txt = r.text.xml_content.as_deref().unwrap_or("").to_string();
            s.push_str(&txt);
            rs.push(text_run_from(r, txt));
        }
        // Collapse trivially-styled run lists (e.g. one run with no rPr) so
        // we don't bloat the JSON for plain SST entries.
        if rs.iter().all(is_unstyled_run) {
            rs.clear();
        }
        texts.push(s);
        runs.push(rs);
    }
    (texts, runs)
}

/// Convert one OOXML `<r>` element into our `TextRun`. Properties that
/// aren't set leave the field as `None`/`false` so the renderer can
/// inherit from the cell's own font.
pub(crate) fn text_run_from(r: &xspread::Run, text: String) -> TextRun {
    let mut tr = TextRun {
        text,
        ..Default::default()
    };
    let Some(rpr) = &r.run_properties else {
        return tr;
    };
    // CT_BooleanProperty: element present + no `val` attr defaults to true,
    // but `val="0"` explicitly unsets the property. Same pattern as Font.
    if let Some(b) = rpr.x_b.first() {
        tr.bold = b.val.unwrap_or(true);
    }
    if let Some(i) = rpr.x_i.first() {
        tr.italic = i.val.unwrap_or(true);
    }
    if let Some(u) = rpr.x_u.first() {
        // OOXML CT_UnderlineProperty: element present, no `val` => `single`
        // (default). `val="none"` explicitly disables underline; all other
        // values turn it on, with the variant captured in `underline_style`.
        let variant = underline_variant(u.val);
        match variant {
            Some("none") => {}
            Some(v) => {
                tr.underline = true;
                if v != "single" {
                    tr.underline_style = Some(v.to_string());
                }
            }
            None => {
                tr.underline = true;
            }
        }
    }
    if let Some(s) = rpr.x_strike.first() {
        tr.strike = s.val.unwrap_or(true);
    }
    if let Some(sz) = rpr.x_sz.first() {
        tr.size = Some(sz.val as f32);
    }
    if let Some(rf) = rpr.x_r_font.first() {
        tr.font_name = Some(rf.val.as_str().to_string());
    }
    if let Some(c) = rpr.x_color.first() {
        let any = c.rgb.is_some() || c.theme.is_some() || c.indexed.is_some();
        if any {
            tr.color = Some(Color {
                rgb: c.rgb.as_ref().map(|s| s.as_str().to_string()),
                theme: c.theme,
                indexed: c.indexed,
                tint: c.tint,
            });
        }
    }
    // OOXML `<vertAlign val="superscript|subscript|baseline"/>`. Baseline
    // is the default — omit so the field stays absent in JSON.
    if let Some(v) = rpr.x_vert_align.first() {
        tr.vert_align = vert_align_variant(v.val);
    }
    // `<family val="N"/>` — OOXML clamps 0..5. Stored as `Option<u8>` so
    // the renderer can pick a CSS fallback (serif / sans-serif / etc.)
    // when the named typeface isn't installed.
    if let Some(fm) = rpr.x_family.first() {
        let v = fm.val;
        if (0..=5).contains(&v) {
            tr.family = Some(v as u8);
        }
    }
    // `<scheme val="major|minor"/>` — theme font reference. `none` is
    // omitted to match the OOXML default.
    if let Some(s) = rpr.x_scheme.first() {
        tr.scheme = font_scheme_variant(s.val);
    }
    tr
}

/// Map ooxmlsdk's `FontSchemeValues` to a wire string. `None`/`"none"`
/// returns `None` so the field is omitted (matches the OOXML default and
/// keeps the JSON small).
pub(crate) fn font_scheme_variant(v: xspread::FontSchemeValues) -> Option<String> {
    use xspread::FontSchemeValues as S;
    match v {
        S::None => None,
        S::Major => Some("major".to_string()),
        S::Minor => Some("minor".to_string()),
    }
}

/// Map ooxmlsdk's `VerticalAlignmentRunValues` to a wire string.
/// `Baseline` returns `None` so the field is omitted (matches the OOXML
/// default and keeps JSON tidy).
pub(crate) fn vert_align_variant(v: xspread::VerticalAlignmentRunValues) -> Option<String> {
    use xspread::VerticalAlignmentRunValues as V;
    match v {
        V::Baseline => None,
        V::Superscript => Some("superscript".to_string()),
        V::Subscript => Some("subscript".to_string()),
    }
}

/// Map an `Option<UnderlineValues>` from ooxmlsdk to one of the OOXML
/// `<u val="..."/>` strings. `None` (no `val` attr) returns `None` and
/// the caller treats it as the default `single`.
pub(crate) fn underline_variant(v: Option<xspread::UnderlineValues>) -> Option<&'static str> {
    use xspread::UnderlineValues as U;
    let v = v?;
    Some(match v {
        U::Single => "single",
        U::Double => "double",
        U::SingleAccounting => "singleAccounting",
        U::DoubleAccounting => "doubleAccounting",
        U::None => "none",
    })
}

fn is_unstyled_run(r: &TextRun) -> bool {
    !r.bold
        && !r.italic
        && !r.underline
        && !r.strike
        && r.size.is_none()
        && r.font_name.is_none()
        && r.color.is_none()
        && r.vert_align.is_none()
        && r.family.is_none()
        && r.scheme.is_none()
}
