use crate::schema::*;
use crate::shapes_style::{apply_font_ref_to_runs, resolve_style_refs};
use crate::shapes_text::text_body_to_paragraphs;
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
    let xfrm_opt = s.shape_properties.transform2_d.as_ref();
    let (off_opt, ext_opt) = match xfrm_opt {
        Some(x) => (x.offset.as_ref(), x.extents.as_ref()),
        None => (None, None),
    };
    let (off, ext) = match (off_opt, ext_opt) {
        (Some(o), Some(e)) => (o, e),
        _ => {
            if parent.is_some() {
                return None;
            }
            return Some(WorldBox {
                x: 0.0,
                y: 0.0,
                cx: 1.0,
                cy: 1.0,
            });
        }
    };
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

#[derive(Clone, Copy, Debug)]
struct ShapeAnchor {
    world: WorldBox,
    #[allow(dead_code)]
    preset: Option<&'static str>,
}

fn collect_shape_anchors(
    root: &ShapeTreeRoot<'_>,
    out: &mut std::collections::HashMap<u32, ShapeAnchor>,
) {
    match root {
        ShapeTreeRoot::Sp(s) => collect_from_shape(s, None, out),
        ShapeTreeRoot::GrpSp(g) => collect_from_group(g, None, out),
        ShapeTreeRoot::CxnSp(_) => {}
    }
}

fn collect_from_shape(
    s: &xdr::Shape,
    parent: Option<GroupFrame>,
    out: &mut std::collections::HashMap<u32, ShapeAnchor>,
) {
    let Some(world) = shape_world(s, parent) else {
        return;
    };
    let id = s
        .non_visual_shape_properties
        .non_visual_drawing_properties
        .id;
    out.insert(
        id,
        ShapeAnchor {
            world,
            preset: None,
        },
    );
}

fn collect_from_group(
    g: &xdr::GroupShape,
    parent: Option<GroupFrame>,
    out: &mut std::collections::HashMap<u32, ShapeAnchor>,
) {
    let Some((_, frame)) = group_frame(g, parent) else {
        return;
    };
    for choice in &g.group_shape_choice {
        match choice {
            xdr::GroupShapeChoice::XdrSp(s) => collect_from_shape(s, Some(frame), out),
            xdr::GroupShapeChoice::XdrGrpSp(inner) => collect_from_group(inner, Some(frame), out),
            _ => {}
        }
    }
}

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

    let mut anchors: std::collections::HashMap<u32, ShapeAnchor> = std::collections::HashMap::new();
    collect_shape_anchors(&root, &mut anchors);

    match root {
        ShapeTreeRoot::Sp(s) => {
            visit_shape(s, None, outer, &mut nodes, theme);
        }
        ShapeTreeRoot::GrpSp(g) => {
            visit_group(g, None, outer, &mut nodes, theme, images, &anchors);
        }
        ShapeTreeRoot::CxnSp(c) => {
            visit_connector(c, None, outer, &mut nodes, theme, &anchors);
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
    let xfrm_opt = sp.transform2_d.as_ref();
    let (off_opt, ext_opt) = match xfrm_opt {
        Some(x) => (x.offset.as_ref(), x.extents.as_ref()),
        None => (None, None),
    };
    let (off, ext) = match (off_opt, ext_opt) {
        (Some(o), Some(e)) => (o, e),
        _ => {
            if parent.is_some() {
                return None;
            }
            return Some(WorldBox {
                x: 0.0,
                y: 0.0,
                cx: 1.0,
                cy: 1.0,
            });
        }
    };
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
    anchors: &std::collections::HashMap<u32, ShapeAnchor>,
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
                visit_group(inner, Some(frame), outer, nodes, theme, images, anchors);
            }
            xdr::GroupShapeChoice::XdrPic(pic) => {
                visit_picture(pic, Some(frame), outer, nodes, images);
            }
            xdr::GroupShapeChoice::XdrCxnSp(c) => {
                visit_connector(c, Some(frame), outer, nodes, theme, anchors);
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
        adj2: None,
        elbow_axis: None,
        fill_gradient: None,
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
    let mut fill = solid_fill_color(&sp.shape_properties_choice2, theme);
    let fill_gradient = gradient_fill(&sp.shape_properties_choice2, theme);
    let (mut outline_color, mut outline_width_emu) = outline_info(sp.a_ln.as_deref(), theme);
    let (text_anchor, text_wrap, text_insets_emu, mut paragraphs) =
        text_body_to_paragraphs(s.text_body.as_deref(), theme);
    let rotation = sp.transform2_d.as_ref().and_then(|x| x.rotation);
    let flip_h = sp
        .transform2_d
        .as_ref()
        .and_then(|x| x.horizontal_flip)
        .unwrap_or(false);
    let flip_v = sp
        .transform2_d
        .as_ref()
        .and_then(|x| x.vertical_flip)
        .unwrap_or(false);

    let preset_is_line = preset
        .as_deref()
        .map(|p| matches!(p, "line" | "lineInv"))
        .unwrap_or(false);
    if let Some(refs) = resolve_style_refs(s.shape_style.as_deref(), theme) {
        if fill.is_none() && fill_gradient.is_none() && !preset_is_line {
            fill = refs.fill;
        }
        if outline_color.is_none() {
            outline_color = refs.outline;
        }
        if outline_width_emu.is_none() {
            outline_width_emu = refs.outline_width_emu;
        }
        apply_font_ref_to_runs(&mut paragraphs, &refs.font_name, &refs.font_color);
    }

    let has_paint = fill.is_some() || fill_gradient.is_some() || outline_color.is_some();
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
        flip_h: if flip_h { Some(true) } else { None },
        flip_v: if flip_v { Some(true) } else { None },
        line_dash: None,
        is_connector: None,
        head_end: None,
        tail_end: None,
        adj1: preset_adj1(sp),
        adj2: preset_adj2(sp),
        elbow_axis: None,
        fill_gradient,
    });
}

fn preset_geom_name(sp: &xdr::ShapeProperties) -> Option<String> {
    use xdr::ShapePropertiesChoice;
    match sp.shape_properties_choice1.as_ref()? {
        ShapePropertiesChoice::APrstGeom(g) => Some(g.preset.as_xml_str().to_string()),
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

fn resolve_gradient_stop_color(
    c: &a::GradientStopChoice,
    theme: Option<&Theme>,
) -> Option<String> {
    use a::GradientStopChoice as G;
    match c {
        G::ASrgbClr(c) => {
            let dbg = format!("{:?}", c);
            let v: &str = &c.val;
            if v.len() == 6 {
                Some(crate::chart_colors::apply_color_modifiers(
                    &format!("#{}", v),
                    &dbg,
                ))
            } else {
                None
            }
        }
        G::ASchemeClr(c) => {
            let dbg = format!("{:?}", c);
            crate::chart_colors::theme_scheme_color(&dbg, theme)
                .map(|base| crate::chart_colors::apply_color_modifiers(&base, &dbg))
        }
        G::APrstClr(c) => {
            let dbg = format!("{:?}", c.val);
            preset_color_hex(&dbg).map(|s| s.to_string())
        }
        G::ASysClr(c) => c.last_color.as_deref().map(|s| format!("#{}", s)),
        _ => None,
    }
}

fn rect_pct(v: Option<i32>) -> f64 {
    (v.unwrap_or(0) as f64) / 100_000.0
}

fn gradient_fill(
    choice: &Option<xdr::ShapePropertiesChoice2>,
    theme: Option<&Theme>,
) -> Option<ShapeGradient> {
    use xdr::ShapePropertiesChoice2;
    let gf = match choice.as_ref()? {
        ShapePropertiesChoice2::AGradFill(g) => g,
        _ => return None,
    };
    let mut stops: Vec<ShapeGradientStop> = Vec::new();
    if let Some(gs_lst) = gf.gradient_stop_list.as_ref() {
        for gs in &gs_lst.a_gs {
            let pos = (gs.position as f32) / 100_000.0;
            let color = match gs.gradient_stop_choice.as_ref() {
                Some(c) => resolve_gradient_stop_color(c, theme),
                None => None,
            };
            if let Some(color) = color {
                stops.push(ShapeGradientStop { pos, color });
            }
        }
    }
    if stops.len() < 2 {
        return None;
    }

    use a::GradientFillChoice as GC;
    match gf.gradient_fill_choice.as_ref() {
        Some(GC::ALin(lin)) => {
            let ang = lin.angle.unwrap_or(0);

            let angle_deg = (ang as f64) / 60_000.0;
            Some(ShapeGradient {
                stops,
                kind: "linear".to_string(),
                angle_deg: Some(angle_deg),
                path: None,
                fill_to_rect: None,
            })
        }
        Some(GC::APath(p)) => {
            let path = p
                .path
                .as_ref()
                .map(|v| format!("{:?}", v).to_ascii_lowercase());
            let ftr = p.fill_to_rectangle.as_ref().map(|r| {
                vec![
                    rect_pct(r.left),
                    rect_pct(r.top),
                    rect_pct(r.right),
                    rect_pct(r.bottom),
                ]
            });
            Some(ShapeGradient {
                stops,
                kind: "path".to_string(),
                angle_deg: None,
                path,
                fill_to_rect: ftr,
            })
        }
        None => Some(ShapeGradient {
            stops,
            kind: "linear".to_string(),
            angle_deg: Some(0.0),
            path: None,
            fill_to_rect: None,
        }),
    }
}

pub(crate) fn resolve_solid_fill(sf: &a::SolidFill, theme: Option<&Theme>) -> Option<String> {
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

pub(crate) fn preset_color_hex(variant_dbg: &str) -> Option<&'static str> {
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

fn preset_adj_n(sp: &xdr::ShapeProperties, target: &[&str]) -> Option<i32> {
    use xdr::ShapePropertiesChoice;
    let geom = match sp.shape_properties_choice1.as_ref()? {
        ShapePropertiesChoice::APrstGeom(g) => g,
        _ => return None,
    };
    let avl = geom.adjust_value_list.as_ref()?;
    for gd in &avl.a_gd {
        let name: &str = &gd.name;
        if !target.iter().any(|t| *t == name) {
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

fn preset_adj1(sp: &xdr::ShapeProperties) -> Option<i32> {
    preset_adj_n(sp, &["adj1", "adj"])
}

fn preset_adj2(sp: &xdr::ShapeProperties) -> Option<i32> {
    preset_adj_n(sp, &["adj2"])
}

fn is_vert_site(idx: u32) -> bool {
    idx == 0 || idx == 2
}

fn is_horiz_site(idx: u32) -> bool {
    idx == 1 || idx == 3
}

fn connection_site(bbox: WorldBox, idx: u32) -> (f64, f64) {
    let cx = bbox.x + bbox.cx / 2.0;
    let cy = bbox.y + bbox.cy / 2.0;
    match idx {
        0 => (cx, bbox.y),
        1 => (bbox.x + bbox.cx, cy),
        2 => (cx, bbox.y + bbox.cy),
        3 => (bbox.x, cy),
        _ => (cx, cy),
    }
}

fn visit_connector(
    c: &xdr::ConnectionShape,
    parent: Option<GroupFrame>,
    outer: WorldBox,
    nodes: &mut Vec<ShapeNode>,
    theme: Option<&Theme>,
    anchors: &std::collections::HashMap<u32, ShapeAnchor>,
) {
    let sp = &c.shape_properties;
    let xfrm_world = match connector_world(sp, parent) {
        Some(w) => w,
        None => return,
    };

    let cxn_pr = &c
        .non_visual_connection_shape_properties
        .non_visual_connector_shape_drawing_properties;
    let resolved_start = cxn_pr.start_connection.as_ref().and_then(|s| {
        anchors
            .get(&s.id)
            .map(|a| connection_site(a.world, s.index))
    });
    let resolved_end = cxn_pr.end_connection.as_ref().and_then(|e| {
        anchors
            .get(&e.id)
            .map(|a| connection_site(a.world, e.index))
    });

    let elbow_axis: Option<String> = match (
        cxn_pr.start_connection.as_ref().map(|s| s.index),
        cxn_pr.end_connection.as_ref().map(|e| e.index),
    ) {
        (Some(s), Some(e)) if is_vert_site(s) && is_vert_site(e) => Some("vertical".to_string()),
        (Some(s), Some(e)) if is_horiz_site(s) && is_horiz_site(e) => {
            Some("horizontal".to_string())
        }
        _ => None,
    };

    let mut override_flip_h: Option<bool> = None;
    let mut override_flip_v: Option<bool> = None;
    let mut override_rotation: Option<i32> = None;
    let world = match (resolved_start, resolved_end) {
        (Some((sx, sy)), Some((ex, ey))) => {
            let min_x = sx.min(ex);
            let max_x = sx.max(ex);
            let min_y = sy.min(ey);
            let max_y = sy.max(ey);
            let cx = (max_x - min_x).max(1.0);
            let cy = (max_y - min_y).max(1.0);
            override_flip_h = Some(sx > ex);
            override_flip_v = Some(sy > ey);
            override_rotation = Some(0);
            WorldBox {
                x: min_x,
                y: min_y,
                cx,
                cy,
            }
        }
        _ => xfrm_world,
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
    let (mut outline_color, mut outline_width_emu) = outline_info(ln_box, theme);
    if outline_color.is_none() || outline_width_emu.is_none() {
        if let Some(refs) = resolve_style_refs(c.shape_style.as_deref(), theme) {
            if outline_color.is_none() {
                outline_color = refs.outline;
            }
            if outline_width_emu.is_none() {
                outline_width_emu = refs.outline_width_emu;
            }
        }
    }
    let dash = line_dash_token(ln_box);
    let head_end = ln_box
        .and_then(|ln| ln.a_head_end.as_ref())
        .and_then(|e| line_end_to_schema(e.r#type.as_ref(), e.width.as_ref(), e.length.as_ref()));
    let tail_end = ln_box
        .and_then(|ln| ln.a_tail_end.as_ref())
        .and_then(|e| line_end_to_schema(e.r#type.as_ref(), e.width.as_ref(), e.length.as_ref()));
    let xfrm = sp.transform2_d.as_ref();
    let flip_h =
        override_flip_h.unwrap_or_else(|| xfrm.and_then(|x| x.horizontal_flip).unwrap_or(false));
    let flip_v =
        override_flip_v.unwrap_or_else(|| xfrm.and_then(|x| x.vertical_flip).unwrap_or(false));
    let rotation = override_rotation.or_else(|| xfrm.and_then(|x| x.rotation));
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
        adj2: None,
        elbow_axis,
        fill_gradient: None,
    });
}
