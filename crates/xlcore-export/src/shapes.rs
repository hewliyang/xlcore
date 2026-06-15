use crate::schema::*;
use crate::shapes_connector::{
    connector_world, preset_adj1, preset_adj2, preset_adj3, visit_connector,
};
use crate::shapes_fill::{
    gradient_fill, line_cap_token, line_dash_token, line_end_to_schema, line_join_token,
    outer_shadow, outline_info, solid_fill_color,
};
use crate::shapes_style::{apply_font_ref_to_runs, resolve_style_refs};
use crate::shapes_text::text_body_to_paragraphs;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;

#[derive(Clone, Copy, Debug)]
pub(crate) struct WorldBox {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) cx: f64,
    pub(crate) cy: f64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Frame {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    tx: f64,
    ty: f64,
}

impl Frame {
    const IDENTITY: Frame = Frame {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.a * x + self.c * y + self.tx,
            self.b * x + self.d * y + self.ty,
        )
    }

    fn compose(self, other: Frame) -> Frame {
        Frame {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            tx: self.a * other.tx + self.c * other.ty + self.tx,
            ty: self.b * other.tx + self.d * other.ty + self.ty,
        }
    }

    fn translation(tx: f64, ty: f64) -> Frame {
        Frame {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            tx,
            ty,
        }
    }

    fn rotation(theta_rad: f64) -> Frame {
        let (sn, cs) = theta_rad.sin_cos();
        Frame {
            a: cs,
            b: sn,
            c: -sn,
            d: cs,
            tx: 0.0,
            ty: 0.0,
        }
    }

    fn scale(sx: f64, sy: f64) -> Frame {
        Frame {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            tx: 0.0,
            ty: 0.0,
        }
    }

    fn scale_x_mag(&self) -> f64 {
        (self.a * self.a + self.b * self.b).sqrt()
    }

    fn scale_y_mag(&self) -> f64 {
        (self.c * self.c + self.d * self.d).sqrt()
    }

    fn rotation_rad(&self) -> f64 {
        if self.scale_x_mag() < 1e-9 {
            0.0
        } else {
            self.b.atan2(self.a)
        }
    }
}

fn frame_or_identity(f: Option<Frame>) -> Frame {
    f.unwrap_or(Frame::IDENTITY)
}

fn rad_to_xfrm_rot(rad: f64) -> i32 {
    let deg = rad.to_degrees();
    (deg * 60000.0).round() as i32
}

pub(crate) fn merge_rotation(parent_rad: f64, own: Option<i32>) -> Option<i32> {
    let own_v = own.unwrap_or(0);
    let parent_v = rad_to_xfrm_rot(parent_rad);
    let total = own_v.wrapping_add(parent_v);
    if total == 0 {
        None
    } else {
        Some(total)
    }
}

fn group_frame(g: &xdr::GroupShape, parent: Option<Frame>) -> Option<(WorldBox, Frame)> {
    let xfrm = g.group_shape_properties.transform_group.as_ref()?;
    let off = xfrm.offset.as_ref()?;
    let ext = xfrm.extents.as_ref()?;
    let ch_off = xfrm.child_offset.as_ref()?;
    let ch_ext = xfrm.child_extents.as_ref()?;
    let own_off_x = off.x.to_emu() as f64;
    let own_off_y = off.y.to_emu() as f64;
    let own_ext_cx = ext.cx.to_emu() as f64;
    let own_ext_cy = ext.cy.to_emu() as f64;
    let ch_off_x = ch_off.x.to_emu() as f64;
    let ch_off_y = ch_off.y.to_emu() as f64;
    let ch_ext_cx = ch_ext.cx.to_emu() as f64;
    let ch_ext_cy = ch_ext.cy.to_emu() as f64;
    let sx = if ch_ext_cx > 0.0 {
        own_ext_cx / ch_ext_cx
    } else {
        1.0
    };
    let sy = if ch_ext_cy > 0.0 {
        own_ext_cy / ch_ext_cy
    } else {
        1.0
    };
    let rot_rad = xfrm
        .rotation
        .map(|r| (r as f64 / 60000.0).to_radians())
        .unwrap_or(0.0);

    let local = Frame::translation(own_off_x + own_ext_cx / 2.0, own_off_y + own_ext_cy / 2.0)
        .compose(Frame::rotation(rot_rad))
        .compose(Frame::translation(-own_ext_cx / 2.0, -own_ext_cy / 2.0))
        .compose(Frame::scale(sx, sy))
        .compose(Frame::translation(-ch_off_x, -ch_off_y));
    let frame = match parent {
        Some(p) => p.compose(local),
        None => local,
    };

    let bbox_parent_local = match parent {
        Some(p) => {
            let (x, y) = p.apply(own_off_x, own_off_y);
            let dx = p.scale_x_mag();
            let dy = p.scale_y_mag();
            WorldBox {
                x,
                y,
                cx: own_ext_cx * dx,
                cy: own_ext_cy * dy,
            }
        }
        None => WorldBox {
            x: own_off_x,
            y: own_off_y,
            cx: own_ext_cx,
            cy: own_ext_cy,
        },
    };
    Some((bbox_parent_local, frame))
}

fn shape_world(s: &xdr::Shape, parent: Option<Frame>) -> Option<(WorldBox, f64)> {
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
            return Some((
                WorldBox {
                    x: 0.0,
                    y: 0.0,
                    cx: 1.0,
                    cy: 1.0,
                },
                0.0,
            ));
        }
    };
    Some(transform_local_box(
        parent,
        off.x.to_emu() as f64,
        off.y.to_emu() as f64,
        ext.cx.to_emu() as f64,
        ext.cy.to_emu() as f64,
    ))
}

pub(crate) fn transform_local_box(
    parent: Option<Frame>,
    off_x: f64,
    off_y: f64,
    ext_cx: f64,
    ext_cy: f64,
) -> (WorldBox, f64) {
    let f = frame_or_identity(parent);
    let (cx_world, cy_world) = f.apply(off_x + ext_cx / 2.0, off_y + ext_cy / 2.0);
    let sx = f.scale_x_mag();
    let sy = f.scale_y_mag();
    let w = ext_cx * sx;
    let h = ext_cy * sy;
    (
        WorldBox {
            x: cx_world - w / 2.0,
            y: cy_world - h / 2.0,
            cx: w,
            cy: h,
        },
        f.rotation_rad(),
    )
}

pub(crate) type ImageUriResolver<'a> = &'a dyn Fn(&str) -> Option<String>;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ShapeAnchor {
    pub(crate) world: WorldBox,
    #[allow(dead_code)]
    pub(crate) preset: Option<&'static str>,
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
    parent: Option<Frame>,
    out: &mut std::collections::HashMap<u32, ShapeAnchor>,
) {
    let Some((world, _rot)) = shape_world(s, parent) else {
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
    parent: Option<Frame>,
    out: &mut std::collections::HashMap<u32, ShapeAnchor>,
) {
    let Some((_, frame)) = group_frame(g, parent) else {
        return;
    };
    for choice in &g.group_shape_choice {
        match choice {
            xdr::GroupShapeChoice::Shape(s) => collect_from_shape(s, Some(frame), out),
            xdr::GroupShapeChoice::GroupShape(inner) => collect_from_group(inner, Some(frame), out),
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
        ShapeTreeRoot::Sp(s) => shape_world(s, None).map(|(w, _)| w)?,
        ShapeTreeRoot::GrpSp(g) => group_frame(g, None).map(|(w, _)| w)?,
        ShapeTreeRoot::CxnSp(c) => connector_world(&c.shape_properties, None).map(|(w, _)| w)?,
    };

    if outer.cx <= 0.0 && outer.cy <= 0.0 {
        return None;
    }

    let mut anchors: std::collections::HashMap<u32, ShapeAnchor> = std::collections::HashMap::new();
    collect_shape_anchors(&root, &mut anchors);

    match root {
        ShapeTreeRoot::Sp(s) => {
            visit_shape(s, None, outer, &mut nodes, theme, images);
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

fn visit_group(
    g: &xdr::GroupShape,
    parent: Option<Frame>,
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
            xdr::GroupShapeChoice::Shape(s) => {
                visit_shape(s, Some(frame), outer, nodes, theme, images);
            }
            xdr::GroupShapeChoice::GroupShape(inner) => {
                visit_group(inner, Some(frame), outer, nodes, theme, images, anchors);
            }
            xdr::GroupShapeChoice::Picture(pic) => {
                visit_picture(pic, Some(frame), outer, nodes, images);
            }
            xdr::GroupShapeChoice::ConnectionShape(c) => {
                visit_connector(c, Some(frame), outer, nodes, theme, anchors);
            }

            _ => {}
        }
    }
}

fn visit_picture(
    pic: &xdr::Picture,
    parent: Option<Frame>,
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
    let (world, parent_rot_rad) = transform_local_box(
        parent,
        off.x.to_emu() as f64,
        off.y.to_emu() as f64,
        ext.cx.to_emu() as f64,
        ext.cy.to_emu() as f64,
    );
    if world.cx <= 0.0 || world.cy <= 0.0 {
        return;
    }
    let blip_fill = match pic.blip_fill.as_ref() {
        Some(b) => b,
        None => return,
    };
    let blip = match blip_fill.blip.as_ref() {
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
    let src = blip_fill.source_rectangle.as_ref();
    let pct_i32 = |v: Option<ooxmlsdk::simple_type::DrawingmlPercentageValue>| -> i32 {
        v.map(|p| p.as_drawingml_percent()).unwrap_or(0)
    };
    let crop = src.map(|r| {
        vec![
            pct_i32(r.left),
            pct_i32(r.top),
            pct_i32(r.right),
            pct_i32(r.bottom),
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
        rotation: merge_rotation(parent_rot_rad, xfrm.rotation),
        paragraphs: Vec::new(),
        text_wrap: None,
        text_insets_emu: None,
        image_data_uri: Some(data_uri),
        image_src_rect: crop,
        flip_h: None,
        flip_v: None,
        line_dash: None,
        line_cap: None,
        line_join: None,
        is_connector: None,
        head_end: None,
        tail_end: None,
        adj1: None,
        adj2: None,
        adj3: None,
        elbow_axis: None,
        fill_gradient: None,
        outer_shadow: None,
        text_autofit: None,
        text_font_scale: None,
        text_line_space_reduction: None,
        text_rotation: None,
        text_vert: None,
        text_vert_overflow: None,
        text_horz_overflow: None,
        fill_blip: None,
    });
}

fn visit_shape(
    s: &xdr::Shape,
    parent: Option<Frame>,
    outer: WorldBox,
    nodes: &mut Vec<ShapeNode>,
    theme: Option<&Theme>,
    images: ImageUriResolver<'_>,
) {
    let (world, parent_rot_rad) = match shape_world(s, parent) {
        Some(v) => v,
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
    let mut fill_gradient = gradient_fill(&sp.shape_properties_choice2, theme);
    let fill_blip = blip_fill(&sp.shape_properties_choice2, images);
    let mut outer_shadow = outer_shadow(&sp.shape_properties_choice3, theme);
    let (mut outline_color, mut outline_width_emu) = outline_info(sp.outline.as_deref(), theme);
    let mut line_dash: Option<String> = line_dash_token(sp.outline.as_deref());
    let mut line_cap: Option<String> = line_cap_token(sp.outline.as_deref());
    let mut line_join: Option<String> = line_join_token(sp.outline.as_deref());
    let head_end = sp
        .outline
        .as_deref()
        .and_then(|ln| ln.head_end.as_ref())
        .and_then(|e| line_end_to_schema(e.r#type.as_ref(), e.width.as_ref(), e.length.as_ref()));
    let tail_end = sp
        .outline
        .as_deref()
        .and_then(|ln| ln.tail_end.as_ref())
        .and_then(|e| line_end_to_schema(e.r#type.as_ref(), e.width.as_ref(), e.length.as_ref()));
    let tb_out = text_body_to_paragraphs(s.text_body.as_deref(), theme);
    let text_anchor = tb_out.anchor;
    let text_wrap = tb_out.wrap;
    let text_insets_emu = tb_out.insets;
    let text_autofit = tb_out.autofit_kind;
    let text_font_scale = tb_out.autofit_font_scale;
    let text_line_space_reduction = tb_out.autofit_line_space_reduction;
    let text_rotation = tb_out.rotation;
    let text_vert = tb_out.vert;
    let text_vert_overflow = tb_out.vert_overflow;
    let text_horz_overflow = tb_out.horz_overflow;
    let mut paragraphs = tb_out.paragraphs;
    let rotation = merge_rotation(
        parent_rot_rad,
        sp.transform2_d.as_ref().and_then(|x| x.rotation),
    );
    let flip_h = sp
        .transform2_d
        .as_ref()
        .and_then(|x| x.horizontal_flip)
        .unwrap_or(false.into());
    let flip_v = sp
        .transform2_d
        .as_ref()
        .and_then(|x| x.vertical_flip)
        .unwrap_or(false.into());

    let preset_is_line = preset
        .as_deref()
        .map(|p| matches!(p, "line" | "lineInv"))
        .unwrap_or(false);
    if let Some(refs) = resolve_style_refs(s.shape_style.as_deref(), theme) {
        if fill.is_none() && fill_gradient.is_none() && !preset_is_line {
            if refs.fill_gradient.is_some() {
                fill_gradient = refs.fill_gradient;
            } else {
                fill = refs.fill;
            }
        }
        if outline_color.is_none() {
            outline_color = refs.outline;
        }
        if outline_width_emu.is_none() {
            outline_width_emu = refs.outline_width_emu;
        }
        if line_dash.is_none() {
            line_dash = refs.line_dash;
        }
        if line_cap.is_none() {
            line_cap = refs.line_cap;
        }
        if line_join.is_none() {
            line_join = refs.line_join;
        }
        if outer_shadow.is_none() {
            outer_shadow = refs.outer_shadow;
        }
        apply_font_ref_to_runs(&mut paragraphs, &refs.font_name, &refs.font_color);
    }

    let has_paint =
        fill.is_some() || fill_gradient.is_some() || fill_blip.is_some() || outline_color.is_some();
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
        flip_h: if flip_h.into() { Some(true) } else { None },
        flip_v: if flip_v.into() { Some(true) } else { None },
        line_dash,
        line_cap,
        line_join,
        is_connector: None,
        head_end,
        tail_end,
        adj1: preset_adj1(sp),
        adj2: preset_adj2(sp),
        adj3: preset_adj3(sp),
        elbow_axis: None,
        fill_gradient,
        outer_shadow,
        text_autofit,
        text_font_scale,
        text_line_space_reduction,
        text_rotation,
        text_vert,
        text_vert_overflow,
        text_horz_overflow,
        fill_blip,
    });
}

fn blip_fill(
    choice: &Option<xdr::ShapePropertiesChoice2>,
    images: ImageUriResolver<'_>,
) -> Option<ShapeBlipFill> {
    use xdr::ShapePropertiesChoice2;
    let bf = match choice.as_ref()? {
        ShapePropertiesChoice2::BlipFill(b) => b,
        _ => return None,
    };
    let blip = bf.blip.as_ref()?;

    let mut data_uri: Option<String> = None;
    if let Some(ext_lst) = blip.blip_extension_list.as_ref() {
        for ext in &ext_lst.blip_extension {
            if let Some(a::BlipExtensionChoice::SvgBlip(sv)) = ext.blip_extension_choice.as_ref() {
                if let Some(embed) = sv.embed.as_deref() {
                    if let Some(uri) = images(embed) {
                        data_uri = Some(uri);
                        break;
                    }
                }
            }
        }
    }
    if data_uri.is_none() {
        let embed = blip.embed.as_deref()?;
        data_uri = images(embed);
    }
    let data_uri = data_uri?;

    let src_rect = bf.source_rectangle.as_ref().and_then(|r| {
        let pct = |v: Option<ooxmlsdk::simple_type::DrawingmlPercentageValue>| -> i32 {
            v.map(|p| p.as_drawingml_percent()).unwrap_or(0)
        };
        let v = vec![pct(r.left), pct(r.top), pct(r.right), pct(r.bottom)];
        if v.iter().any(|n| *n != 0) {
            Some(v)
        } else {
            None
        }
    });

    let kind = bf.blip_fill_choice.as_ref().map(|c| match c {
        a::BlipFillChoice::Tile(_) => "tile".to_string(),
        a::BlipFillChoice::Stretch(_) => "stretch".to_string(),
    });

    Some(ShapeBlipFill {
        data_uri,
        src_rect,
        kind,
    })
}

pub(crate) fn preset_geom_name(sp: &xdr::ShapeProperties) -> Option<String> {
    use xdr::ShapePropertiesChoice;
    match sp.shape_properties_choice1.as_ref()? {
        ShapePropertiesChoice::PresetGeometry(g) => Some(g.preset.as_xml_str().to_string()),
        ShapePropertiesChoice::CustomGeometry(_) => None,
    }
}
pub(crate) fn resolve_solid_fill(sf: &a::SolidFill, theme: Option<&Theme>) -> Option<String> {
    use a::SolidFillChoice;
    match sf.solid_fill_choice.as_ref()? {
        SolidFillChoice::RgbColorModelHex(c) => {
            let v: &str = &c.val;
            if v.len() == 6 {
                Some(format!("#{}", v))
            } else {
                None
            }
        }
        SolidFillChoice::SchemeColor(c) => {
            let dbg = format!("{:?}", c);
            crate::chart_colors::theme_scheme_color(&dbg, theme)
                .map(|base| crate::chart_colors::apply_color_modifiers(&base, &dbg))
        }
        SolidFillChoice::PresetColor(c) => {
            let dbg = format!("{:?}", c.val);
            preset_color_hex(&dbg).map(|s| s.to_string())
        }
        SolidFillChoice::SystemColor(c) => {
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

pub(crate) fn prst_dash_token(v: &a::PresetLineDashValues) -> &'static str {
    use a::PresetLineDashValues as P;
    match v {
        P::Solid => "solid",
        P::Dot => "dot",
        P::Dash => "dash",
        P::LargeDash => "lgDash",
        P::DashDot => "dashDot",
        P::LargeDashDot => "lgDashDot",
        P::LargeDashDotDot => "lgDashDotDot",
        P::SystemDash => "sysDash",
        P::SystemDot => "sysDot",
        P::SystemDashDot => "sysDashDot",
        P::SystemDashDotDot => "sysDashDotDot",
    }
}
