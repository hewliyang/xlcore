use crate::schema::*;
use crate::shapes::resolve_solid_fill;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;

pub(crate) fn text_body_to_paragraphs(
    tb: Option<&xdr::TextBody>,
    theme: Option<&Theme>,
) -> (
    Option<String>,
    Option<String>,
    Option<Vec<i32>>,
    Vec<ShapeParagraph>,
) {
    let Some(tb) = tb else {
        return (None, None, None, Vec::new());
    };
    let anchor = body_anchor_token(&tb.body_properties);
    let wrap = body_wrap_token(&tb.body_properties);
    let insets = body_insets_emu(&tb.body_properties);

    let list_style = tb.list_style.as_deref();

    let mut paragraphs: Vec<ShapeParagraph> = Vec::new();
    for p in &tb.a_p {
        let p_pr = p.paragraph_properties.as_deref();
        let level = p_pr.and_then(|pp| pp.level).unwrap_or(0).clamp(0, 8) as usize;

        let align = pick_align(p_pr, list_style, level);

        let mut runs: Vec<TextRun> = Vec::new();
        for ch in &p.paragraph_choice {
            match ch {
                a::ParagraphChoice::AR(run) => {
                    let text: &str = &run.text;
                    if text.is_empty() {
                        continue;
                    }
                    let mut tr = TextRun {
                        text: text.to_string(),
                        ..Default::default()
                    };
                    apply_lst_style_def_p_pr(list_style, &mut tr, theme);
                    apply_lst_style_lvl_p_pr(list_style, level, &mut tr, theme);
                    apply_pp_def_r_pr(p_pr, &mut tr, theme);
                    if let Some(rp) = run.run_properties.as_deref() {
                        apply_run_properties(rp, &mut tr, theme);
                    }
                    runs.push(tr);
                }
                a::ParagraphChoice::ABr(_) => {
                    runs.push(TextRun {
                        text: "\n".to_string(),
                        ..Default::default()
                    });
                }
                a::ParagraphChoice::AFld(field) => {
                    let text = match field.text.as_deref() {
                        Some(s) if !s.is_empty() => s.to_string(),
                        _ => continue,
                    };
                    let mut tr = TextRun {
                        text,
                        ..Default::default()
                    };
                    apply_lst_style_def_p_pr(list_style, &mut tr, theme);
                    apply_lst_style_lvl_p_pr(list_style, level, &mut tr, theme);
                    apply_pp_def_r_pr(p_pr, &mut tr, theme);
                    if let Some(rp) = field.run_properties.as_deref() {
                        apply_run_properties(rp, &mut tr, theme);
                    }
                    runs.push(tr);
                }
                _ => {}
            }
        }
        if !runs.is_empty() {
            paragraphs.push(ShapeParagraph { align, runs });
        }
    }
    (anchor, wrap, insets, paragraphs)
}

fn pick_align(
    p_pr: Option<&a::ParagraphProperties>,
    list_style: Option<&a::ListStyle>,
    level: usize,
) -> Option<String> {
    let mut align: Option<String> = None;
    if let Some(ls) = list_style {
        if let Some(def_pp) = ls.default_paragraph_properties.as_deref() {
            if let Some(s) = alignment_token(&def_pp.alignment) {
                align = Some(s);
            }
        }
        if let Some(lvl_pp) = lvl_paragraph_alignment(ls, level) {
            if let Some(s) = lvl_pp {
                align = Some(s);
            }
        }
    }
    if let Some(s) = paragraph_align_token(p_pr) {
        align = Some(s);
    }
    align
}

fn apply_lst_style_def_p_pr(
    list_style: Option<&a::ListStyle>,
    tr: &mut TextRun,
    theme: Option<&Theme>,
) {
    let Some(ls) = list_style else { return };
    let Some(def_pp) = ls.default_paragraph_properties.as_deref() else {
        return;
    };
    if let Some(dr) = def_pp.a_def_r_pr.as_deref() {
        apply_default_run_properties(dr, tr, theme);
    }
}

fn apply_lst_style_lvl_p_pr(
    list_style: Option<&a::ListStyle>,
    level: usize,
    tr: &mut TextRun,
    theme: Option<&Theme>,
) {
    let Some(ls) = list_style else { return };
    if let Some(dr) = lvl_paragraph_def_r_pr(ls, level) {
        apply_default_run_properties(dr, tr, theme);
    }
}

fn apply_pp_def_r_pr(
    p_pr: Option<&a::ParagraphProperties>,
    tr: &mut TextRun,
    theme: Option<&Theme>,
) {
    let Some(pp) = p_pr else { return };
    if let Some(dr) = pp.a_def_r_pr.as_deref() {
        apply_default_run_properties(dr, tr, theme);
    }
}

fn lvl_paragraph_def_r_pr(ls: &a::ListStyle, level: usize) -> Option<&a::DefaultRunProperties> {
    match level {
        0 => ls
            .level1_paragraph_properties
            .as_deref()
            .and_then(|p| p.a_def_r_pr.as_deref()),
        1 => ls
            .level2_paragraph_properties
            .as_deref()
            .and_then(|p| p.a_def_r_pr.as_deref()),
        2 => ls
            .level3_paragraph_properties
            .as_deref()
            .and_then(|p| p.a_def_r_pr.as_deref()),
        3 => ls
            .level4_paragraph_properties
            .as_deref()
            .and_then(|p| p.a_def_r_pr.as_deref()),
        4 => ls
            .level5_paragraph_properties
            .as_deref()
            .and_then(|p| p.a_def_r_pr.as_deref()),
        5 => ls
            .level6_paragraph_properties
            .as_deref()
            .and_then(|p| p.a_def_r_pr.as_deref()),
        6 => ls
            .level7_paragraph_properties
            .as_deref()
            .and_then(|p| p.a_def_r_pr.as_deref()),
        7 => ls
            .level8_paragraph_properties
            .as_deref()
            .and_then(|p| p.a_def_r_pr.as_deref()),
        8 => ls
            .level9_paragraph_properties
            .as_deref()
            .and_then(|p| p.a_def_r_pr.as_deref()),
        _ => None,
    }
}

fn lvl_paragraph_alignment(ls: &a::ListStyle, level: usize) -> Option<Option<String>> {
    let align = match level {
        0 => ls
            .level1_paragraph_properties
            .as_deref()
            .map(|p| alignment_token(&p.alignment)),
        1 => ls
            .level2_paragraph_properties
            .as_deref()
            .map(|p| alignment_token(&p.alignment)),
        2 => ls
            .level3_paragraph_properties
            .as_deref()
            .map(|p| alignment_token(&p.alignment)),
        3 => ls
            .level4_paragraph_properties
            .as_deref()
            .map(|p| alignment_token(&p.alignment)),
        4 => ls
            .level5_paragraph_properties
            .as_deref()
            .map(|p| alignment_token(&p.alignment)),
        5 => ls
            .level6_paragraph_properties
            .as_deref()
            .map(|p| alignment_token(&p.alignment)),
        6 => ls
            .level7_paragraph_properties
            .as_deref()
            .map(|p| alignment_token(&p.alignment)),
        7 => ls
            .level8_paragraph_properties
            .as_deref()
            .map(|p| alignment_token(&p.alignment)),
        8 => ls
            .level9_paragraph_properties
            .as_deref()
            .map(|p| alignment_token(&p.alignment)),
        _ => None,
    };
    align
}

fn alignment_token(alignment: &Option<a::TextAlignmentTypeValues>) -> Option<String> {
    let dbg = format!("{:?}", alignment);
    if !dbg.starts_with("Some(") {
        return None;
    }
    if dbg.contains("Center") {
        Some("ctr".to_string())
    } else if dbg.contains("Right") {
        Some("r".to_string())
    } else if dbg.contains("Justified") {
        Some("just".to_string())
    } else if dbg.contains("Left") {
        Some("l".to_string())
    } else {
        None
    }
}

fn body_insets_emu(bp: &a::BodyProperties) -> Option<Vec<i32>> {
    let l = bp.left_inset;
    let t = bp.top_inset;
    let r = bp.right_inset;
    let b = bp.bottom_inset;
    if l.is_none() && t.is_none() && r.is_none() && b.is_none() {
        return None;
    }
    const DEF_LR: i32 = 91440;
    const DEF_TB: i32 = 45720;
    Some(vec![
        l.unwrap_or(DEF_LR),
        t.unwrap_or(DEF_TB),
        r.unwrap_or(DEF_LR),
        b.unwrap_or(DEF_TB),
    ])
}

fn body_wrap_token(bp: &a::BodyProperties) -> Option<String> {
    let dbg = format!("{:?}", bp.wrap);
    if !dbg.starts_with("Some(") {
        return None;
    }
    if dbg.contains("None_") || dbg.contains("NoWrap") {
        Some("none".to_string())
    } else if dbg.contains("Square") {
        Some("square".to_string())
    } else {
        None
    }
}

fn body_anchor_token(bp: &a::BodyProperties) -> Option<String> {
    let dbg = format!("{:?}", bp.anchor);

    if dbg.contains("Center") {
        Some("ctr".to_string())
    } else if dbg.contains("Bottom") {
        Some("b".to_string())
    } else if dbg.contains("Top") {
        Some("t".to_string())
    } else {
        None
    }
}

fn paragraph_align_token(pp: Option<&a::ParagraphProperties>) -> Option<String> {
    let pp = pp?;
    alignment_token(&pp.alignment)
}

fn underline_is_visible(u: Option<&a::TextUnderlineValues>) -> bool {
    matches!(u, Some(v) if !matches!(v, a::TextUnderlineValues::None))
}

fn strike_is_visible(s: Option<&a::TextStrikeValues>) -> bool {
    matches!(s, Some(v) if !matches!(v, a::TextStrikeValues::NoStrike))
}

fn apply_run_properties(rp: &a::RunProperties, tr: &mut TextRun, theme: Option<&Theme>) {
    let solid_fill = match rp.run_properties_choice1.as_ref() {
        Some(a::RunPropertiesChoice::ASolidFill(sf)) => Some(sf.as_ref()),
        _ => None,
    };
    apply_run_fields(
        tr,
        theme,
        rp.font_size,
        rp.bold,
        rp.italic,
        underline_is_visible(rp.underline.as_ref()),
        strike_is_visible(rp.strike.as_ref()),
        solid_fill,
        rp.a_latin.as_ref(),
    );
}

fn apply_default_run_properties(
    rp: &a::DefaultRunProperties,
    tr: &mut TextRun,
    theme: Option<&Theme>,
) {
    let solid_fill = match rp.default_run_properties_choice1.as_ref() {
        Some(a::DefaultRunPropertiesChoice::ASolidFill(sf)) => Some(sf.as_ref()),
        _ => None,
    };
    apply_run_fields(
        tr,
        theme,
        rp.font_size,
        rp.bold,
        rp.italic,
        underline_is_visible(rp.underline.as_ref()),
        strike_is_visible(rp.strike.as_ref()),
        solid_fill,
        rp.a_latin.as_ref(),
    );
}

fn apply_run_fields(
    tr: &mut TextRun,
    theme: Option<&Theme>,
    font_size: Option<i32>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline_present: bool,
    strike_present: bool,
    solid_fill: Option<&a::SolidFill>,
    latin: Option<&a::LatinFont>,
) {
    if let Some(sz) = font_size {
        tr.size = Some((sz as f32) / 100.0);
    }
    if let Some(b) = bold {
        tr.bold = b;
    }
    if let Some(i) = italic {
        tr.italic = i;
    }
    if underline_present {
        tr.underline = true;
    }
    if strike_present {
        tr.strike = true;
    }
    if let Some(sf) = solid_fill {
        if let Some(hex) = resolve_solid_fill(sf, theme) {
            let stripped = hex.trim_start_matches('#');
            if stripped.len() == 6 {
                tr.color = Some(Color {
                    rgb: Some(stripped.to_string()),
                    theme: None,
                    indexed: None,
                    tint: None,
                });
            }
        }
    }
    if let Some(latin) = latin {
        let tf: &str = latin.typeface.as_deref().unwrap_or("");
        if !tf.is_empty() && !tf.starts_with('+') {
            tr.font_name = Some(tf.to_string());
        } else if tf == "+mn-lt" {
            if let Some(t) = theme {
                tr.font_name = t.minor_font.clone();
            }
        } else if tf == "+mj-lt" {
            if let Some(t) = theme {
                tr.font_name = t.major_font.clone();
            }
        }
    }
}
