use crate::schema::*;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;

#[derive(Clone, Copy, Debug)]
struct WorldBox {
    x: f64,
    y: f64,
    cx: f64,
    cy: f64,
}

#[derive(Clone, Copy, Debug)]
struct GroupFrame {
    parent_world: WorldBox,
    child_off_x: f64,
    child_off_y: f64,
    child_ext_cx: f64,
    child_ext_cy: f64,
}

impl GroupFrame {
    fn map(&self, off_x: f64, off_y: f64, ext_cx: f64, ext_cy: f64) -> WorldBox {
        let sx = if self.child_ext_cx > 0.0 {
            self.parent_world.cx / self.child_ext_cx
        } else {
            1.0
        };
        let sy = if self.child_ext_cy > 0.0 {
            self.parent_world.cy / self.child_ext_cy
        } else {
            1.0
        };
        WorldBox {
            x: self.parent_world.x + (off_x - self.child_off_x) * sx,
            y: self.parent_world.y + (off_y - self.child_off_y) * sy,
            cx: ext_cx * sx,
            cy: ext_cy * sy,
        }
    }
}

fn group_frame(g: &xdr::GroupShape, parent: Option<GroupFrame>) -> Option<(WorldBox, GroupFrame)> {
    let xfrm = g
        .group_shape_properties
        .as_ref()?
        .transform_group
        .as_ref()?;
    let off = xfrm.offset.as_ref()?;
    let ext = xfrm.extents.as_ref()?;
    let ch_off = xfrm.child_offset.as_ref()?;
    let ch_ext = xfrm.child_extents.as_ref()?;
    let own_off_x = off.x as f64;
    let own_off_y = off.y as f64;
    let own_ext_cx = ext.cx as f64;
    let own_ext_cy = ext.cy as f64;

    let world = match parent {
        Some(p) => p.map(own_off_x, own_off_y, own_ext_cx, own_ext_cy),
        None => WorldBox {
            x: own_off_x,
            y: own_off_y,
            cx: own_ext_cx,
            cy: own_ext_cy,
        },
    };
    let frame = GroupFrame {
        parent_world: world,
        child_off_x: ch_off.x as f64,
        child_off_y: ch_off.y as f64,
        child_ext_cx: ch_ext.cx as f64,
        child_ext_cy: ch_ext.cy as f64,
    };
    Some((world, frame))
}

fn shape_world(s: &xdr::Shape, parent: Option<GroupFrame>) -> Option<WorldBox> {
    let xfrm = s.shape_properties.transform2_d.as_ref()?;
    let off = xfrm.offset.as_ref()?;
    let ext = xfrm.extents.as_ref()?;
    let ox = off.x as f64;
    let oy = off.y as f64;
    let cx = ext.cx as f64;
    let cy = ext.cy as f64;
    Some(match parent {
        Some(p) => p.map(ox, oy, cx, cy),
        None => WorldBox {
            x: ox,
            y: oy,
            cx,
            cy,
        },
    })
}

pub(crate) type ImageUriResolver<'a> = &'a dyn Fn(&str) -> Option<String>;

pub(crate) fn extract_shape_tree(
    root: ShapeTreeRoot<'_>,
    theme: Option<&Theme>,
    images: ImageUriResolver<'_>,
) -> Option<Shape> {
    let mut nodes: Vec<ShapeNode> = Vec::new();

    let outer = match &root {
        ShapeTreeRoot::Sp(s) => shape_world(s, None)?,
        ShapeTreeRoot::GrpSp(g) => group_frame(g, None).map(|(w, _)| w)?,
        ShapeTreeRoot::CxnSp(c) => connector_world(&c.shape_properties, None)?,
    };

    if outer.cx <= 0.0 && outer.cy <= 0.0 {
        return None;
    }
    match root {
        ShapeTreeRoot::Sp(s) => {
            visit_shape(s, None, outer, &mut nodes, theme);
        }
        ShapeTreeRoot::GrpSp(g) => {
            visit_group(g, None, outer, &mut nodes, theme, images);
        }
        ShapeTreeRoot::CxnSp(c) => {
            visit_connector(c, None, outer, &mut nodes, theme);
        }
    }
    if nodes.is_empty() {
        None
    } else {
        Some(Shape { nodes })
    }
}

pub(crate) enum ShapeTreeRoot<'a> {
    Sp(&'a xdr::Shape),
    GrpSp(&'a xdr::GroupShape),
    CxnSp(&'a xdr::ConnectionShape),
}

fn connector_world(sp: &xdr::ShapeProperties, parent: Option<GroupFrame>) -> Option<WorldBox> {
    let xfrm = sp.transform2_d.as_ref()?;
    let off = xfrm.offset.as_ref()?;
    let ext = xfrm.extents.as_ref()?;
    let ox = off.x as f64;
    let oy = off.y as f64;
    let cx = ext.cx as f64;
    let cy = ext.cy as f64;
    Some(match parent {
        Some(p) => p.map(ox, oy, cx, cy),
        None => WorldBox {
            x: ox,
            y: oy,
            cx,
            cy,
        },
    })
}

fn visit_group(
    g: &xdr::GroupShape,
    parent: Option<GroupFrame>,
    outer: WorldBox,
    nodes: &mut Vec<ShapeNode>,
    theme: Option<&Theme>,
    images: ImageUriResolver<'_>,
) {
    let (_, frame) = match group_frame(g, parent) {
        Some(v) => v,
        None => return,
    };
    for choice in &g.group_shape_choice {
        match choice {
            xdr::GroupShapeChoice::XdrSp(s) => {
                visit_shape(s, Some(frame), outer, nodes, theme);
            }
            xdr::GroupShapeChoice::XdrGrpSp(inner) => {
                visit_group(inner, Some(frame), outer, nodes, theme, images);
            }
            xdr::GroupShapeChoice::XdrPic(pic) => {
                visit_picture(pic, Some(frame), outer, nodes, images);
            }
            xdr::GroupShapeChoice::XdrCxnSp(c) => {
                visit_connector(c, Some(frame), outer, nodes, theme);
            }

            _ => {}
        }
    }
}

fn visit_picture(
    pic: &xdr::Picture,
    parent: Option<GroupFrame>,
    outer: WorldBox,
    nodes: &mut Vec<ShapeNode>,
    images: ImageUriResolver<'_>,
) {
    let xfrm = match pic.shape_properties.transform2_d.as_ref() {
        Some(x) => x,
        None => return,
    };
    let (off, ext) = match (xfrm.offset.as_ref(), xfrm.extents.as_ref()) {
        (Some(o), Some(e)) => (o, e),
        _ => return,
    };
    let world = match parent {
        Some(p) => p.map(off.x as f64, off.y as f64, ext.cx as f64, ext.cy as f64),
        None => WorldBox {
            x: off.x as f64,
            y: off.y as f64,
            cx: ext.cx as f64,
            cy: ext.cy as f64,
        },
    };
    if world.cx <= 0.0 || world.cy <= 0.0 {
        return;
    }
    let blip = match pic.blip_fill.blip.as_ref() {
        Some(b) => b,
        None => return,
    };
    let embed: &str = match blip.embed.as_deref() {
        Some(s) => s,
        None => return,
    };
    let data_uri = match images(embed) {
        Some(u) => u,
        None => return,
    };
    let src = pic.blip_fill.source_rectangle.as_ref();
    let crop = src.map(|r| {
        vec![
            r.left.unwrap_or(0),
            r.top.unwrap_or(0),
            r.right.unwrap_or(0),
            r.bottom.unwrap_or(0),
        ]
    });

    let crop = crop.filter(|v| v.iter().any(|n| *n != 0));

    let rel_x = if outer.cx > 0.0 {
        (world.x - outer.x) / outer.cx
    } else {
        0.0
    };
    let rel_y = if outer.cy > 0.0 {
        (world.y - outer.y) / outer.cy
    } else {
        0.0
    };
    let rel_w = if outer.cx > 0.0 {
        world.cx / outer.cx
    } else {
        1.0
    };
    let rel_h = if outer.cy > 0.0 {
        world.cy / outer.cy
    } else {
        1.0
    };

    nodes.push(ShapeNode {
        rel_x: rel_x as f32,
        rel_y: rel_y as f32,
        rel_w: rel_w as f32,
        rel_h: rel_h as f32,
        preset: None,
        fill: None,
        outline_color: None,
        outline_width_emu: None,
        text_anchor: None,
        rotation: xfrm.rotation,
        paragraphs: Vec::new(),
        text_wrap: None,
        text_insets_emu: None,
        image_data_uri: Some(data_uri),
        image_src_rect: crop,
        flip_h: None,
        flip_v: None,
        line_dash: None,
        is_connector: None,
        head_end: None,
        tail_end: None,
        adj1: None,
    });
}

fn visit_shape(
    s: &xdr::Shape,
    parent: Option<GroupFrame>,
    outer: WorldBox,
    nodes: &mut Vec<ShapeNode>,
    theme: Option<&Theme>,
) {
    let world = match shape_world(s, parent) {
        Some(w) => w,

        None => return,
    };
    if world.cx <= 0.0 || world.cy <= 0.0 {
        return;
    }
    let rel_x = if outer.cx > 0.0 {
        (world.x - outer.x) / outer.cx
    } else {
        0.0
    };
    let rel_y = if outer.cy > 0.0 {
        (world.y - outer.y) / outer.cy
    } else {
        0.0
    };
    let rel_w = if outer.cx > 0.0 {
        world.cx / outer.cx
    } else {
        1.0
    };
    let rel_h = if outer.cy > 0.0 {
        world.cy / outer.cy
    } else {
        1.0
    };

    let sp = &s.shape_properties;
    let preset = preset_geom_name(sp);
    let fill = solid_fill_color(&sp.shape_properties_choice2, theme);
    let (outline_color, outline_width_emu) = outline_info(sp.a_ln.as_deref(), theme);
    let (text_anchor, text_wrap, text_insets_emu, paragraphs) =
        text_body_to_paragraphs(s.text_body.as_deref(), theme);
    let rotation = sp.transform2_d.as_ref().and_then(|x| x.rotation);

    let has_paint = fill.is_some() || outline_color.is_some();
    let has_text = !paragraphs.is_empty();
    if !has_paint && !has_text {
        return;
    }

    nodes.push(ShapeNode {
        rel_x: rel_x as f32,
        rel_y: rel_y as f32,
        rel_w: rel_w as f32,
        rel_h: rel_h as f32,
        preset,
        fill,
        outline_color,
        outline_width_emu,
        text_anchor,
        rotation,
        paragraphs,
        text_wrap,
        text_insets_emu,
        image_data_uri: None,
        image_src_rect: None,
        flip_h: None,
        flip_v: None,
        line_dash: None,
        is_connector: None,
        head_end: None,
        tail_end: None,
        adj1: None,
    });
}

fn preset_geom_name(sp: &xdr::ShapeProperties) -> Option<String> {
    use xdr::ShapePropertiesChoice;
    match sp.shape_properties_choice1.as_ref()? {
        ShapePropertiesChoice::APrstGeom(g) => {
            let dbg = format!("{:?}", g.preset);
            let mut chars = dbg.chars();
            let first = chars.next()?;
            let rest: String = chars.collect();
            Some(format!("{}{}", first.to_ascii_lowercase(), rest))
        }
        ShapePropertiesChoice::ACustGeom(_) => None,
    }
}

fn solid_fill_color(
    choice: &Option<xdr::ShapePropertiesChoice2>,
    theme: Option<&Theme>,
) -> Option<String> {
    use xdr::ShapePropertiesChoice2;
    match choice.as_ref()? {
        ShapePropertiesChoice2::ASolidFill(sf) => resolve_solid_fill(sf, theme),

        _ => None,
    }
}

fn resolve_solid_fill(sf: &a::SolidFill, theme: Option<&Theme>) -> Option<String> {
    use a::SolidFillChoice;
    match sf.solid_fill_choice.as_ref()? {
        SolidFillChoice::ASrgbClr(c) => {
            let v: &str = &c.val;
            if v.len() == 6 {
                Some(format!("#{}", v))
            } else {
                None
            }
        }
        SolidFillChoice::ASchemeClr(c) => {
            let dbg = format!("{:?}", c);
            crate::chart_colors::theme_scheme_color(&dbg, theme)
                .map(|base| crate::chart_colors::apply_color_modifiers(&base, &dbg))
        }
        SolidFillChoice::APrstClr(c) => {
            let dbg = format!("{:?}", c.val);
            preset_color_hex(&dbg).map(|s| s.to_string())
        }
        SolidFillChoice::ASysClr(c) => {
            let last: Option<&str> = c.last_color.as_deref();
            last.map(|s| format!("#{}", s))
        }

        _ => None,
    }
}

fn preset_color_hex(variant_dbg: &str) -> Option<&'static str> {
    Some(match variant_dbg {
        "Black" => "#000000",
        "White" => "#FFFFFF",
        "Red" => "#FF0000",
        "Green" => "#008000",
        "Blue" => "#0000FF",
        "Yellow" => "#FFFF00",
        "Orange" => "#FFA500",
        "Purple" => "#800080",
        "Gray" | "Grey" => "#808080",
        "DarkRed" => "#8B0000",
        "DarkGreen" => "#006400",
        "DarkBlue" => "#00008B",
        "LightGray" | "LightGrey" => "#D3D3D3",
        "DarkGray" | "DarkGrey" => "#A9A9A9",
        _ => return None,
    })
}

fn outline_info(ln: Option<&a::Outline>, theme: Option<&Theme>) -> (Option<String>, Option<i32>) {
    let Some(ln) = ln else {
        return (None, None);
    };
    let width = ln.width;
    use a::OutlineChoice;
    let color = match ln.outline_choice1.as_ref() {
        Some(OutlineChoice::ASolidFill(sf)) => resolve_solid_fill(sf, theme),
        Some(OutlineChoice::ANoFill(_)) => None,
        _ => None,
    };
    (color, width)
}

fn line_dash_token(ln: Option<&a::Outline>) -> Option<String> {
    let ln = ln?;
    use a::OutlineChoice2;
    match ln.outline_choice2.as_ref()? {
        OutlineChoice2::APrstDash(d) => {
            let dbg = format!("{:?}", d.val);

            if !dbg.starts_with("Some(") {
                return None;
            }
            let inner = dbg.trim_start_matches("Some(").trim_end_matches(')');
            let mut chars = inner.chars();
            let first = chars.next()?;
            let rest: String = chars.collect();
            Some(format!("{}{}", first.to_ascii_lowercase(), rest))
        }
        _ => None,
    }
}

fn line_end_to_schema(
    kind: Option<&a::LineEndValues>,
    w: Option<&a::LineEndWidthValues>,
    len: Option<&a::LineEndLengthValues>,
) -> Option<LineEnd> {
    let kind_tok = kind.and_then(|v| enum_token(&format!("{:?}", v)));
    let w_tok = w.and_then(|v| enum_token(&format!("{:?}", v)));
    let len_tok = len.and_then(|v| enum_token(&format!("{:?}", v)));

    if matches!(kind_tok.as_deref(), Some("none")) && w_tok.is_none() && len_tok.is_none() {
        return None;
    }
    if kind_tok.is_none() && w_tok.is_none() && len_tok.is_none() {
        return None;
    }
    Some(LineEnd {
        kind: kind_tok,
        w: w_tok,
        len: len_tok,
    })
}

fn enum_token(dbg: &str) -> Option<String> {
    let trimmed = dbg.trim_end_matches('_');
    if trimmed.is_empty() {
        return None;
    }
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    let rest: String = chars.collect();
    Some(format!("{}{}", first.to_ascii_lowercase(), rest))
}

fn preset_adj1(sp: &xdr::ShapeProperties) -> Option<i32> {
    use xdr::ShapePropertiesChoice;
    let geom = match sp.shape_properties_choice1.as_ref()? {
        ShapePropertiesChoice::APrstGeom(g) => g,
        _ => return None,
    };
    let avl = geom.adjust_value_list.as_ref()?;
    for gd in &avl.a_gd {
        let name: &str = &gd.name;
        if name != "adj1" && name != "adj" {
            continue;
        }
        let fmla: &str = &gd.formula;
        if let Some(rest) = fmla.strip_prefix("val ") {
            if let Ok(v) = rest.trim().parse::<i32>() {
                return Some(v);
            }
        }
    }
    None
}

fn visit_connector(
    c: &xdr::ConnectionShape,
    parent: Option<GroupFrame>,
    outer: WorldBox,
    nodes: &mut Vec<ShapeNode>,
    theme: Option<&Theme>,
) {
    let sp = &c.shape_properties;
    let world = match connector_world(sp, parent) {
        Some(w) => w,
        None => return,
    };

    let rel_x = if outer.cx > 0.0 {
        (world.x - outer.x) / outer.cx
    } else {
        0.0
    };
    let rel_y = if outer.cy > 0.0 {
        (world.y - outer.y) / outer.cy
    } else {
        0.0
    };
    let rel_w = if outer.cx > 0.0 {
        world.cx / outer.cx
    } else {
        1.0
    };
    let rel_h = if outer.cy > 0.0 {
        world.cy / outer.cy
    } else {
        1.0
    };

    let preset = preset_geom_name(sp);
    let ln_box = sp.a_ln.as_deref();
    let (outline_color, outline_width_emu) = outline_info(ln_box, theme);
    let dash = line_dash_token(ln_box);
    let head_end = ln_box
        .and_then(|ln| ln.a_head_end.as_ref())
        .and_then(|e| line_end_to_schema(e.r#type.as_ref(), e.width.as_ref(), e.length.as_ref()));
    let tail_end = ln_box
        .and_then(|ln| ln.a_tail_end.as_ref())
        .and_then(|e| line_end_to_schema(e.r#type.as_ref(), e.width.as_ref(), e.length.as_ref()));
    let xfrm = sp.transform2_d.as_ref();
    let flip_h = xfrm.and_then(|x| x.horizontal_flip).unwrap_or(false);
    let flip_v = xfrm.and_then(|x| x.vertical_flip).unwrap_or(false);
    let rotation = xfrm.and_then(|x| x.rotation);
    let adj1 = preset_adj1(sp);

    let outline_color = outline_color.or_else(|| Some("#000000".to_string()));

    nodes.push(ShapeNode {
        rel_x: rel_x as f32,
        rel_y: rel_y as f32,
        rel_w: rel_w as f32,
        rel_h: rel_h as f32,
        preset,
        fill: None,
        outline_color,
        outline_width_emu,
        text_anchor: None,
        rotation,
        paragraphs: Vec::new(),
        text_wrap: None,
        text_insets_emu: None,
        image_data_uri: None,
        image_src_rect: None,
        flip_h: if flip_h { Some(true) } else { None },
        flip_v: if flip_v { Some(true) } else { None },
        line_dash: dash,
        is_connector: Some(true),
        head_end,
        tail_end,
        adj1,
    });
}

fn text_body_to_paragraphs(
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
    let mut paragraphs: Vec<ShapeParagraph> = Vec::new();
    for p in &tb.a_p {
        let align = paragraph_align_token(p.paragraph_properties.as_deref());
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
                _ => {}
            }
        }
        if !runs.is_empty() {
            paragraphs.push(ShapeParagraph { align, runs });
        }
    }
    (anchor, wrap, insets, paragraphs)
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
    let dbg = format!("{:?}", pp.alignment);

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

fn apply_run_properties(rp: &a::RunProperties, tr: &mut TextRun, theme: Option<&Theme>) {
    if let Some(sz) = rp.font_size {
        tr.size = Some((sz as f32) / 100.0);
    }
    if let Some(b) = rp.bold {
        tr.bold = b;
    }
    if let Some(i) = rp.italic {
        tr.italic = i;
    }
    if let Some(_u) = rp.underline.as_ref() {
        tr.underline = true;
    }
    if let Some(_s) = rp.strike.as_ref() {
        tr.strike = true;
    }

    if let Some(a::RunPropertiesChoice::ASolidFill(sf)) = rp.run_properties_choice1.as_ref() {
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

    if let Some(latin) = rp.a_latin.as_ref() {
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
