use crate::*;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as xspread;

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
    let mut texts = Vec::with_capacity(sst.shared_string_item.len());
    let mut runs = Vec::with_capacity(sst.shared_string_item.len());
    for item in &sst.shared_string_item {
        if let Some(t) = &item.text {
            texts.push(t.xml_content.as_deref().unwrap_or("").to_string());
            runs.push(Vec::new());
            continue;
        }

        let mut s = String::new();
        let mut rs: Vec<TextRun> = Vec::with_capacity(item.run.len());
        for r in &item.run {
            let txt = r.text.xml_content.as_deref().unwrap_or("").to_string();
            s.push_str(&txt);
            rs.push(text_run_from(r, txt));
        }

        if rs.iter().all(is_unstyled_run) {
            rs.clear();
        }
        texts.push(s);
        runs.push(rs);
    }
    (texts, runs)
}

pub(crate) fn text_run_from(r: &xspread::Run, text: String) -> TextRun {
    let mut tr = TextRun {
        text,
        ..Default::default()
    };
    let Some(rpr) = &r.run_properties else {
        return tr;
    };
    let fr = crate::font_flat::flatten_run_properties(rpr);

    if let Some(b) = fr.bold {
        tr.bold = b.val.map(bool::from).unwrap_or(true);
    }
    if let Some(i) = fr.italic {
        tr.italic = i.val.map(bool::from).unwrap_or(true);
    }
    if let Some(u) = fr.underline {
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
    if let Some(s) = fr.strike {
        tr.strike = s.val.map(bool::from).unwrap_or(true);
    }
    if let Some(sz) = fr.font_size {
        tr.size = Some(sz.val as f32);
    }
    if let Some(rf) = fr.run_font {
        tr.font_name = Some(rf.val.as_str().to_string());
    }
    if let Some(c) = fr.color {
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

    if let Some(v) = fr.vertical_text_alignment {
        tr.vert_align = vert_align_variant(v.val);
    }

    if let Some(fm) = fr.font_family {
        let v = fm.val;
        if (0..=5).contains(&v) {
            tr.family = Some(v as u8);
        }
    }

    if let Some(s) = fr.font_scheme {
        tr.scheme = font_scheme_variant(s.val);
    }
    tr
}

pub(crate) fn font_scheme_variant(v: xspread::FontSchemeValues) -> Option<String> {
    use xspread::FontSchemeValues as S;
    match v {
        S::None => None,
        S::Major => Some("major".to_string()),
        S::Minor => Some("minor".to_string()),
    }
}

pub(crate) fn vert_align_variant(v: xspread::VerticalAlignmentRunValues) -> Option<String> {
    use xspread::VerticalAlignmentRunValues as V;
    match v {
        V::Baseline => None,
        V::Superscript => Some("superscript".to_string()),
        V::Subscript => Some("subscript".to_string()),
    }
}

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
