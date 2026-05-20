use crate::schema::*;
use crate::shapes::{
    Frame, ShapeAnchor, WorldBox, merge_rotation, preset_geom_name, transform_local_box,
};
use crate::shapes_fill::{
    line_cap_token, line_dash_token, line_end_to_schema, line_join_token, outline_info,
};
use crate::shapes_style::resolve_style_refs;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;

pub(crate) fn connector_world(
    sp: &xdr::ShapeProperties,
    parent: Option<Frame>,
) -> Option<(WorldBox, f64)> {
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
        off.x as f64,
        off.y as f64,
        ext.cx as f64,
        ext.cy as f64,
    ))
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

pub(crate) fn preset_adj1(sp: &xdr::ShapeProperties) -> Option<i32> {
    preset_adj_n(sp, &["adj1", "adj"])
}

pub(crate) fn preset_adj2(sp: &xdr::ShapeProperties) -> Option<i32> {
    preset_adj_n(sp, &["adj2"])
}

pub(crate) fn preset_adj3(sp: &xdr::ShapeProperties) -> Option<i32> {
    preset_adj_n(sp, &["adj3"])
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

pub(crate) fn visit_connector(
    c: &xdr::ConnectionShape,
    parent: Option<Frame>,
    outer: WorldBox,
    nodes: &mut Vec<ShapeNode>,
    theme: Option<&Theme>,
    anchors: &std::collections::HashMap<u32, ShapeAnchor>,
) {
    let sp = &c.shape_properties;
    let (xfrm_world, parent_rot_rad) = match connector_world(sp, parent) {
        Some(v) => v,
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
    let mut dash = line_dash_token(ln_box);
    let mut line_cap = line_cap_token(ln_box);
    let mut line_join = line_join_token(ln_box);
    if outline_color.is_none()
        || outline_width_emu.is_none()
        || dash.is_none()
        || line_cap.is_none()
        || line_join.is_none()
    {
        if let Some(refs) = resolve_style_refs(c.shape_style.as_deref(), theme) {
            if outline_color.is_none() {
                outline_color = refs.outline;
            }
            if outline_width_emu.is_none() {
                outline_width_emu = refs.outline_width_emu;
            }
            if dash.is_none() {
                dash = refs.line_dash;
            }
            if line_cap.is_none() {
                line_cap = refs.line_cap;
            }
            if line_join.is_none() {
                line_join = refs.line_join;
            }
        }
    }
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
    let rotation = match override_rotation {
        Some(r) => merge_rotation(parent_rot_rad, Some(r)),
        None => merge_rotation(parent_rot_rad, xfrm.and_then(|x| x.rotation)),
    };
    let adj1 = preset_adj1(sp);
    let adj2 = preset_adj2(sp);
    let adj3 = preset_adj3(sp);

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
        line_cap,
        line_join,
        is_connector: Some(true),
        head_end,
        tail_end,
        adj1,
        adj2,
        adj3,
        elbow_axis,
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
