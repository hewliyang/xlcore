use crate::schema::*;
use crate::shapes::resolve_solid_fill;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;

pub(crate) struct TextBodyOut {
    pub anchor: Option<String>,
    pub wrap: Option<String>,
    pub insets: Option<Vec<i32>>,
    pub autofit_kind: Option<String>,
    pub autofit_font_scale: Option<i32>,
    pub autofit_line_space_reduction: Option<i32>,
    pub rotation: Option<i32>,
    pub vert: Option<String>,
    pub vert_overflow: Option<String>,
    pub horz_overflow: Option<String>,
    pub paragraphs: Vec<ShapeParagraph>,
}

impl TextBodyOut {
    fn empty() -> Self {
        Self {
            anchor: None,
            wrap: None,
            insets: None,
            autofit_kind: None,
            autofit_font_scale: None,
            autofit_line_space_reduction: None,
            rotation: None,
            vert: None,
            vert_overflow: None,
            horz_overflow: None,
            paragraphs: Vec::new(),
        }
    }
}

#[derive(Default, Clone, Copy)]
struct LnSp {
    pct: Option<i32>,
    pts: Option<i32>,
}

#[derive(Clone)]
struct BulletKindResolved {
    kind: &'static str,
    char: Option<String>,
    auto_num_type: Option<String>,
    auto_num_start_at: Option<i32>,
}

#[derive(Default, Clone)]
struct PpResolved<'a> {
    align: Option<String>,
    mar_l: Option<i32>,
    indent: Option<i32>,
    level: Option<u8>,
    line_spacing: LnSp,
    space_before: LnSp,
    space_after: LnSp,
    bullet_kind: Option<BulletKindResolved>,
    bullet_color: Option<&'a a::BulletColor>,
    bullet_font: Option<&'a a::BulletFont>,
    bullet_size_pct: Option<i32>,
    bullet_size_pts: Option<i32>,
    def_r_pr: Option<&'a a::DefaultRunProperties>,
}

macro_rules! pp_view {
    (
        $vis:vis $name:ident,
        $ty:ty,
        ($f1:ident, $c1:ident),
        ($f2:ident, $c2:ident),
        ($f3:ident, $c3:ident),
        ($f4:ident, $c4:ident) $(,)?
    ) => {
        $vis fn $name(p: &$ty) -> PpResolved<'_> {
            PpResolved {
                align: alignment_token(&p.alignment),
                mar_l: p.left_margin,
                indent: p.indent,
                level: p.level.map(|v| v.clamp(0, 8) as u8),
                line_spacing: line_spacing_from(p.line_spacing.as_deref()),
                space_before: space_before_from(p.space_before.as_deref()),
                space_after: space_after_from(p.space_after.as_deref()),
                bullet_color: match p.$f1.as_ref() {
                    Some(a::$c1::ABuClr(c)) => Some(c.as_ref()),
                    _ => None,
                },
                bullet_size_pct: match p.$f2.as_ref() {
                    Some(a::$c2::ABuSzPct(c)) => Some(c.val),
                    _ => None,
                },
                bullet_size_pts: match p.$f2.as_ref() {
                    Some(a::$c2::ABuSzPts(c)) => Some(c.val),
                    _ => None,
                },
                bullet_font: match p.$f3.as_ref() {
                    Some(a::$c3::ABuFont(f)) => Some(f.as_ref()),
                    _ => None,
                },
                bullet_kind: match p.$f4.as_ref() {
                    Some(a::$c4::ABuNone) => Some(BulletKindResolved {
                        kind: "none",
                        char: None,
                        auto_num_type: None,
                        auto_num_start_at: None,
                    }),
                    Some(a::$c4::ABuChar(c)) => Some(BulletKindResolved {
                        kind: "char",
                        char: Some(c.char.to_string()),
                        auto_num_type: None,
                        auto_num_start_at: None,
                    }),
                    Some(a::$c4::ABuAutoNum(c)) => Some(BulletKindResolved {
                        kind: "autoNum",
                        char: None,
                        auto_num_type: Some(auto_num_token(&c.r#type)),
                        auto_num_start_at: c.start_at,
                    }),
                    _ => None,
                },
                def_r_pr: p.a_def_r_pr.as_deref(),
            }
        }
    };
}

#[rustfmt::skip]
pp_view!(
    view_def_pp,
    a::DefaultParagraphProperties,
    (default_paragraph_properties_choice1, DefaultParagraphPropertiesChoice),
    (default_paragraph_properties_choice2, DefaultParagraphPropertiesChoice2),
    (default_paragraph_properties_choice3, DefaultParagraphPropertiesChoice3),
    (default_paragraph_properties_choice4, DefaultParagraphPropertiesChoice4),
);
#[rustfmt::skip]
pp_view!(
    view_pp,
    a::ParagraphProperties,
    (paragraph_properties_choice1, ParagraphPropertiesChoice),
    (paragraph_properties_choice2, ParagraphPropertiesChoice2),
    (paragraph_properties_choice3, ParagraphPropertiesChoice3),
    (paragraph_properties_choice4, ParagraphPropertiesChoice4),
);
#[rustfmt::skip]
pp_view!(
    view_lvl1,
    a::Level1ParagraphProperties,
    (level1_paragraph_properties_choice1, Level1ParagraphPropertiesChoice),
    (level1_paragraph_properties_choice2, Level1ParagraphPropertiesChoice2),
    (level1_paragraph_properties_choice3, Level1ParagraphPropertiesChoice3),
    (level1_paragraph_properties_choice4, Level1ParagraphPropertiesChoice4),
);
#[rustfmt::skip]
pp_view!(
    view_lvl2,
    a::Level2ParagraphProperties,
    (level2_paragraph_properties_choice1, Level2ParagraphPropertiesChoice),
    (level2_paragraph_properties_choice2, Level2ParagraphPropertiesChoice2),
    (level2_paragraph_properties_choice3, Level2ParagraphPropertiesChoice3),
    (level2_paragraph_properties_choice4, Level2ParagraphPropertiesChoice4),
);
#[rustfmt::skip]
pp_view!(
    view_lvl3,
    a::Level3ParagraphProperties,
    (level3_paragraph_properties_choice1, Level3ParagraphPropertiesChoice),
    (level3_paragraph_properties_choice2, Level3ParagraphPropertiesChoice2),
    (level3_paragraph_properties_choice3, Level3ParagraphPropertiesChoice3),
    (level3_paragraph_properties_choice4, Level3ParagraphPropertiesChoice4),
);
#[rustfmt::skip]
pp_view!(
    view_lvl4,
    a::Level4ParagraphProperties,
    (level4_paragraph_properties_choice1, Level4ParagraphPropertiesChoice),
    (level4_paragraph_properties_choice2, Level4ParagraphPropertiesChoice2),
    (level4_paragraph_properties_choice3, Level4ParagraphPropertiesChoice3),
    (level4_paragraph_properties_choice4, Level4ParagraphPropertiesChoice4),
);
#[rustfmt::skip]
pp_view!(
    view_lvl5,
    a::Level5ParagraphProperties,
    (level5_paragraph_properties_choice1, Level5ParagraphPropertiesChoice),
    (level5_paragraph_properties_choice2, Level5ParagraphPropertiesChoice2),
    (level5_paragraph_properties_choice3, Level5ParagraphPropertiesChoice3),
    (level5_paragraph_properties_choice4, Level5ParagraphPropertiesChoice4),
);
#[rustfmt::skip]
pp_view!(
    view_lvl6,
    a::Level6ParagraphProperties,
    (level6_paragraph_properties_choice1, Level6ParagraphPropertiesChoice),
    (level6_paragraph_properties_choice2, Level6ParagraphPropertiesChoice2),
    (level6_paragraph_properties_choice3, Level6ParagraphPropertiesChoice3),
    (level6_paragraph_properties_choice4, Level6ParagraphPropertiesChoice4),
);
#[rustfmt::skip]
pp_view!(
    view_lvl7,
    a::Level7ParagraphProperties,
    (level7_paragraph_properties_choice1, Level7ParagraphPropertiesChoice),
    (level7_paragraph_properties_choice2, Level7ParagraphPropertiesChoice2),
    (level7_paragraph_properties_choice3, Level7ParagraphPropertiesChoice3),
    (level7_paragraph_properties_choice4, Level7ParagraphPropertiesChoice4),
);
#[rustfmt::skip]
pp_view!(
    view_lvl8,
    a::Level8ParagraphProperties,
    (level8_paragraph_properties_choice1, Level8ParagraphPropertiesChoice),
    (level8_paragraph_properties_choice2, Level8ParagraphPropertiesChoice2),
    (level8_paragraph_properties_choice3, Level8ParagraphPropertiesChoice3),
    (level8_paragraph_properties_choice4, Level8ParagraphPropertiesChoice4),
);
#[rustfmt::skip]
pp_view!(
    view_lvl9,
    a::Level9ParagraphProperties,
    (level9_paragraph_properties_choice1, Level9ParagraphPropertiesChoice),
    (level9_paragraph_properties_choice2, Level9ParagraphPropertiesChoice2),
    (level9_paragraph_properties_choice3, Level9ParagraphPropertiesChoice3),
    (level9_paragraph_properties_choice4, Level9ParagraphPropertiesChoice4),
);

fn view_lvl<'a>(ls: &'a a::ListStyle, level: usize) -> Option<PpResolved<'a>> {
    match level {
        0 => ls.level1_paragraph_properties.as_deref().map(view_lvl1),
        1 => ls.level2_paragraph_properties.as_deref().map(view_lvl2),
        2 => ls.level3_paragraph_properties.as_deref().map(view_lvl3),
        3 => ls.level4_paragraph_properties.as_deref().map(view_lvl4),
        4 => ls.level5_paragraph_properties.as_deref().map(view_lvl5),
        5 => ls.level6_paragraph_properties.as_deref().map(view_lvl6),
        6 => ls.level7_paragraph_properties.as_deref().map(view_lvl7),
        7 => ls.level8_paragraph_properties.as_deref().map(view_lvl8),
        8 => ls.level9_paragraph_properties.as_deref().map(view_lvl9),
        _ => None,
    }
}

fn merge_pp<'a>(base: PpResolved<'a>, over: PpResolved<'a>) -> PpResolved<'a> {
    PpResolved {
        align: over.align.or(base.align),
        mar_l: over.mar_l.or(base.mar_l),
        indent: over.indent.or(base.indent),
        level: over.level.or(base.level),
        line_spacing: LnSp {
            pct: over.line_spacing.pct.or(base.line_spacing.pct),
            pts: over.line_spacing.pts.or(base.line_spacing.pts),
        },
        space_before: LnSp {
            pct: over.space_before.pct.or(base.space_before.pct),
            pts: over.space_before.pts.or(base.space_before.pts),
        },
        space_after: LnSp {
            pct: over.space_after.pct.or(base.space_after.pct),
            pts: over.space_after.pts.or(base.space_after.pts),
        },
        bullet_kind: over.bullet_kind.or(base.bullet_kind),
        bullet_color: over.bullet_color.or(base.bullet_color),
        bullet_font: over.bullet_font.or(base.bullet_font),
        bullet_size_pct: over.bullet_size_pct.or(base.bullet_size_pct),
        bullet_size_pts: over.bullet_size_pts.or(base.bullet_size_pts),
        def_r_pr: over.def_r_pr.or(base.def_r_pr),
    }
}

pub(crate) fn text_body_to_paragraphs(
    tb: Option<&xdr::TextBody>,
    theme: Option<&Theme>,
) -> TextBodyOut {
    let Some(tb) = tb else {
        return TextBodyOut::empty();
    };
    let anchor = body_anchor_token(&tb.body_properties);
    let wrap = body_wrap_token(&tb.body_properties);
    let insets = body_insets_emu(&tb.body_properties);
    let (autofit_kind, autofit_font_scale, autofit_line_space_reduction) =
        body_autofit(&tb.body_properties);
    let rotation = tb.body_properties.rotation;
    let vert = body_vert_token(&tb.body_properties);
    let vert_overflow = body_vert_overflow_token(&tb.body_properties);
    let horz_overflow = body_horz_overflow_token(&tb.body_properties);

    let list_style = tb.list_style.as_deref();

    let mut paragraphs: Vec<ShapeParagraph> = Vec::new();
    for p in &tb.a_p {
        let p_pr = p.paragraph_properties.as_deref();
        let level = p_pr.and_then(|pp| pp.level).unwrap_or(0).clamp(0, 8) as usize;

        let mut resolved = PpResolved::default();
        if let Some(ls) = list_style {
            if let Some(def) = ls.default_paragraph_properties.as_deref() {
                resolved = merge_pp(resolved, view_def_pp(def));
            }
            if let Some(lvl) = view_lvl(ls, level) {
                resolved = merge_pp(resolved, lvl);
            }
        }
        if let Some(pp) = p_pr {
            resolved = merge_pp(resolved, view_pp(pp));
        }

        let mut runs: Vec<TextRun> = Vec::new();
        let bake_defaults = |tr: &mut TextRun| {
            if let Some(ls) = list_style {
                if let Some(def) = ls.default_paragraph_properties.as_deref() {
                    if let Some(dr) = def.a_def_r_pr.as_deref() {
                        apply_default_run_properties(dr, tr, theme);
                    }
                }
                if let Some(dr) = lvl_paragraph_def_r_pr(ls, level) {
                    apply_default_run_properties(dr, tr, theme);
                }
            }
            if let Some(pp) = p_pr {
                if let Some(dr) = pp.a_def_r_pr.as_deref() {
                    apply_default_run_properties(dr, tr, theme);
                }
            }
        };

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
                    bake_defaults(&mut tr);
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
                    bake_defaults(&mut tr);
                    if let Some(rp) = field.run_properties.as_deref() {
                        apply_run_properties(rp, &mut tr, theme);
                    }
                    runs.push(tr);
                }
                _ => {}
            }
        }

        if runs.is_empty() {
            continue;
        }

        paragraphs.push(ShapeParagraph {
            align: resolved.align.clone(),
            mar_l_emu: resolved.mar_l,
            indent_emu: resolved.indent,
            level: Some(level as u8),
            line_spacing_pct: resolved.line_spacing.pct,
            line_spacing_pts: resolved.line_spacing.pts,
            space_before_pct: resolved.space_before.pct,
            space_before_pts: resolved.space_before.pts,
            space_after_pct: resolved.space_after.pct,
            space_after_pts: resolved.space_after.pts,
            bullet: build_bullet(&resolved, theme),
            runs,
        });
    }
    TextBodyOut {
        anchor,
        wrap,
        insets,
        autofit_kind,
        autofit_font_scale,
        autofit_line_space_reduction,
        rotation,
        vert,
        vert_overflow,
        horz_overflow,
        paragraphs,
    }
}

fn build_bullet(r: &PpResolved<'_>, theme: Option<&Theme>) -> Option<ShapeBullet> {
    let bk = r.bullet_kind.as_ref()?;
    let color = r.bullet_color.and_then(|c| bullet_color_to_color(c, theme));
    Some(ShapeBullet {
        kind: bk.kind.to_string(),
        char: bk.char.clone(),
        auto_num_type: bk.auto_num_type.clone(),
        auto_num_start_at: bk.auto_num_start_at,
        font: r
            .bullet_font
            .and_then(|f| f.typeface.clone())
            .filter(|s| !s.is_empty()),
        color,
        size_pct: r.bullet_size_pct,
        size_pts: r.bullet_size_pts,
    })
}

fn bullet_color_to_color(bc: &a::BulletColor, theme: Option<&Theme>) -> Option<Color> {
    let hex = match bc.bullet_color_choice.as_ref()? {
        a::BulletColorChoice::ASrgbClr(c) => {
            let v: &str = c.val.as_ref();
            if v.len() == 6 || v.len() == 8 {
                Some(v[v.len() - 6..].to_string())
            } else {
                None
            }
        }
        a::BulletColorChoice::ASchemeClr(c) => resolve_scheme_clr_hex(c, theme),
        a::BulletColorChoice::APrstClr(c) => {
            Some(format!("{:?}", c.val)).map(|_| "808080".to_string())
        }
        a::BulletColorChoice::ASysClr(c) => c
            .last_color
            .as_deref()
            .filter(|s| s.len() == 6)
            .map(|s| s.to_string()),
        _ => None,
    }?;
    if hex.len() != 6 {
        return None;
    }
    Some(Color {
        rgb: Some(hex.to_uppercase()),
        theme: None,
        indexed: None,
        tint: None,
    })
}

fn resolve_scheme_clr_hex(c: &a::SchemeColor, theme: Option<&Theme>) -> Option<String> {
    let theme = theme?;
    let token = format!("{:?}", c.val);
    let idx = scheme_color_index(&token)?;
    theme.colors.get(idx).map(|s| {
        let s = s.trim_start_matches('#');
        s.chars()
            .rev()
            .take(6)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>()
    })
}

fn scheme_color_index(token: &str) -> Option<usize> {
    if token.contains("Light1") || token.contains("Background1") {
        Some(0)
    } else if token.contains("Dark1") || token.contains("Text1") {
        Some(1)
    } else if token.contains("Light2") || token.contains("Background2") {
        Some(2)
    } else if token.contains("Dark2") || token.contains("Text2") {
        Some(3)
    } else if token.contains("Accent1") {
        Some(4)
    } else if token.contains("Accent2") {
        Some(5)
    } else if token.contains("Accent3") {
        Some(6)
    } else if token.contains("Accent4") {
        Some(7)
    } else if token.contains("Accent5") {
        Some(8)
    } else if token.contains("Accent6") {
        Some(9)
    } else if token.contains("Hyperlink") && !token.contains("Followed") {
        Some(10)
    } else if token.contains("FollowedHyperlink") {
        Some(11)
    } else {
        None
    }
}

fn auto_num_token(t: &a::TextAutoNumberSchemeValues) -> String {
    let dbg = format!("{:?}", t);
    if dbg.is_empty() {
        return dbg;
    }
    let mut chars = dbg.chars();
    let first = chars.next().unwrap().to_ascii_lowercase();
    let rest: String = chars.collect();
    format!("{first}{rest}")
}

fn line_spacing_from(ls: Option<&a::LineSpacing>) -> LnSp {
    match ls.and_then(|x| x.line_spacing_choice.as_ref()) {
        Some(a::LineSpacingChoice::ASpcPct(p)) => LnSp {
            pct: Some(p.val),
            pts: None,
        },
        Some(a::LineSpacingChoice::ASpcPts(p)) => LnSp {
            pct: None,
            pts: Some(p.val),
        },
        _ => LnSp::default(),
    }
}

fn space_before_from(s: Option<&a::SpaceBefore>) -> LnSp {
    match s.and_then(|x| x.space_before_choice.as_ref()) {
        Some(a::SpaceBeforeChoice::ASpcPct(p)) => LnSp {
            pct: Some(p.val),
            pts: None,
        },
        Some(a::SpaceBeforeChoice::ASpcPts(p)) => LnSp {
            pct: None,
            pts: Some(p.val),
        },
        _ => LnSp::default(),
    }
}

fn space_after_from(s: Option<&a::SpaceAfter>) -> LnSp {
    match s.and_then(|x| x.space_after_choice.as_ref()) {
        Some(a::SpaceAfterChoice::ASpcPct(p)) => LnSp {
            pct: Some(p.val),
            pts: None,
        },
        Some(a::SpaceAfterChoice::ASpcPts(p)) => LnSp {
            pct: None,
            pts: Some(p.val),
        },
        _ => LnSp::default(),
    }
}

fn body_vert_overflow_token(bp: &a::BodyProperties) -> Option<String> {
    let v = bp.vertical_overflow.as_ref()?;
    use a::TextVerticalOverflowValues as V;
    let s = match v {
        V::Overflow => "overflow",
        V::Ellipsis => "ellipsis",
        V::Clip => "clip",
    };
    Some(s.to_string())
}

fn body_horz_overflow_token(bp: &a::BodyProperties) -> Option<String> {
    let v = bp.horizontal_overflow.as_ref()?;
    use a::TextHorizontalOverflowValues as V;
    let s = match v {
        V::Overflow => "overflow",
        V::Clip => "clip",
    };
    Some(s.to_string())
}

fn body_vert_token(bp: &a::BodyProperties) -> Option<String> {
    let v = bp.vertical.as_ref()?;
    use a::TextVerticalValues as V;
    let s = match v {
        V::Horizontal => return None,
        V::Vertical => "vert",
        V::Vertical270 => "vert270",
        V::WordArtVertical => "wordArtVert",
        V::EastAsianVetical => "eaVert",
        V::MongolianVertical => "mongolianVert",
        V::WordArtLeftToRight => "wordArtVertRtl",
    };
    Some(s.to_string())
}

fn body_autofit(bp: &a::BodyProperties) -> (Option<String>, Option<i32>, Option<i32>) {
    match bp.body_properties_choice1.as_ref() {
        Some(a::BodyPropertiesChoice::ANormAutofit(n)) => (
            Some("norm".to_string()),
            n.font_scale,
            n.line_space_reduction,
        ),
        Some(a::BodyPropertiesChoice::ASpAutoFit(_)) => (Some("shape".to_string()), None, None),
        _ => (None, None, None),
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
    let w = bp.wrap.as_ref()?;
    use a::TextWrappingValues as W;
    let s = match w {
        W::None => "none",
        W::Square => "square",
    };
    Some(s.to_string())
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

fn underline_state(u: Option<&a::TextUnderlineValues>) -> Option<bool> {
    u.map(|v| !matches!(v, a::TextUnderlineValues::None))
}

fn strike_state(s: Option<&a::TextStrikeValues>) -> Option<bool> {
    s.map(|v| !matches!(v, a::TextStrikeValues::NoStrike))
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
        underline_state(rp.underline.as_ref()),
        strike_state(rp.strike.as_ref()),
        solid_fill,
        rp.a_latin.as_ref(),
        rp.kerning,
        rp.baseline,
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
        underline_state(rp.underline.as_ref()),
        strike_state(rp.strike.as_ref()),
        solid_fill,
        rp.a_latin.as_ref(),
        rp.kerning,
        rp.baseline,
    );
}

#[allow(clippy::too_many_arguments)]
fn apply_run_fields(
    tr: &mut TextRun,
    theme: Option<&Theme>,
    font_size: Option<i32>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline_state: Option<bool>,
    strike_state: Option<bool>,
    solid_fill: Option<&a::SolidFill>,
    latin: Option<&a::LatinFont>,
    kern: Option<i32>,
    baseline: Option<i32>,
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
    if let Some(v) = underline_state {
        tr.underline = v;
    }
    if let Some(v) = strike_state {
        tr.strike = v;
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
    if let Some(k) = kern {
        tr.kern = Some(k);
    }
    if let Some(b) = baseline {
        tr.baseline = Some(b);
    }
}
