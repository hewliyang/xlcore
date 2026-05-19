//! Extract `<xdr:sp>` autoshapes and `<xdr:grpSp>` group shapes from
//! a drawing anchor.
//!
//! Excel emits a fair amount of chrome as DrawingML shapes: rounded-
//! rect callouts (the "1 / 2 / 3 / 4" instruction boxes on
//! Microsoft's Map Chart sample template), banners, arrows, sticky
//! notes, "Next >" hyperlink buttons. These show up as `<xdr:sp>`
//! children of `<xdr:twoCellAnchor>` (sometimes wrapped in
//! `<xdr:grpSp>` groups).
//!
//! v0 produces a flattened list of leaf shapes positioned via
//! fractional (0..1) coordinates inside the anchor's bbox. The
//! renderer paints fill + outline + text — unknown `prstGeom` presets
//! fall back to plain rectangle.
//!
//! See ECMA-376 §20.5.2.29 (`<xdr:sp>`) and §20.1.7.5 (xfrm group transform
//! semantics — `xfrm/chOff/chExt` defines the logical→world mapping
//! for nested children).

use crate::schema::*;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;

/// World-space EMU bbox built up by accumulating `xfrm` transforms
/// down a `<xdr:grpSp>` chain. Root frame is the outer-most group's
/// own `xfrm`; if the outer-most container is an `<xdr:sp>` we just
/// use its `spPr/xfrm` directly.
#[derive(Clone, Copy, Debug)]
struct WorldBox {
    x: f64,
    y: f64,
    cx: f64,
    cy: f64,
}

/// One step of the parent group's logical→world mapping.
/// `parent_world` is the parent group's resolved world-EMU bbox;
/// `child_off / child_ext` is its logical-space origin / size.
#[derive(Clone, Copy, Debug)]
struct GroupFrame {
    parent_world: WorldBox,
    child_off_x: f64,
    child_off_y: f64,
    child_ext_cx: f64,
    child_ext_cy: f64,
}

impl GroupFrame {
    /// Map a child's logical-space `(off, ext)` to world-EMU.
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

/// Resolve a `<xdr:grpSp>` to its world-EMU bbox + a child-frame that
/// nested children can use to map their own xfrm into world space.
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
    // Resolve own world bbox: either via parent's frame, or take own
    // xfrm directly when this is the outer-most group (root anchor).
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

/// Resolve an `<xdr:sp>` to its world-EMU bbox. When the shape sits
/// inside a `<xdr:grpSp>`, the parent's `GroupFrame` maps it from
/// logical space; at the top level its own `<a:xfrm>` is already in
/// world coords.
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

/// Public entry: walk a `<xdr:twoCellAnchor>` or `<xdr:oneCellAnchor>`
/// shape child (`<xdr:sp>` or `<xdr:grpSp>`) and produce a `Shape`
/// with fractional bbox children relative to the anchor's resolved
/// pixel rect.
///
/// `anchor_world` is the anchor's EMU bbox derived from the
/// from/to (or from + ext) markers. We need it to normalize world
/// coords back to 0..1 — the renderer's `anchorToRect` is the source
/// of truth for the absolute pixel rect.
/// Walk a top-level shape (or group) and emit a flattened `Shape`
/// with leaf nodes positioned relative to the root shape's own bbox.
///
/// We deliberately use the *shape*'s own xfrm as the outer frame —
/// not the drawing-anchor's cell-anchor bbox — because shape xfrms
/// are stored in world-EMU on the worksheet canvas, and the
/// chart-style cell anchor would require column-width lookup to
/// compute its world bbox here. The renderer maps the resulting
/// fractional coordinates back to the anchor's pixel bbox, which is
/// Excel's effective visual rect for a `twoCellAnchor`-wrapped
/// shape (Excel snaps the shape to the anchor on every save).
/// Resolver from drawing-relationship-id (`r:embed`) to a fully
/// encoded `data:<mime>;base64,...` URI. Caller pre-builds this from
/// the drawings part's image parts. Used so we can surface
/// `<xdr:pic>` nodes nested inside `<xdr:grpSp>` as inline shape
/// children — top-level pictures still route through
/// `AnchorTarget::Image`.
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
    };
    if outer.cx <= 0.0 || outer.cy <= 0.0 {
        return None;
    }
    match root {
        ShapeTreeRoot::Sp(s) => {
            visit_shape(s, None, outer, &mut nodes, theme);
        }
        ShapeTreeRoot::GrpSp(g) => {
            visit_group(g, None, outer, &mut nodes, theme, images);
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
            // graphicFrame / cxnSp inside groups: still out of scope.
            // Connectors would slot in here once we add a generic
            // `line` shape kind to the schema.
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
    // World rect from the picture's own xfrm.
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
    // Drop a crop array of all zeros (the no-op case Excel emits a lot).
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
        // No xfrm at all — Excel usually fills these in even when 0
        // (chartEx fallback shape uses 0/0 ext). Skip the silent
        // "invisible" case rather than rendering a 0×0 rect.
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

    // Skip ornamental nodes that have nothing visible AND no text
    // (Excel emits empty `<xdr:txBox>` shapes as group spacers).
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
    });
}

fn preset_geom_name(sp: &xdr::ShapeProperties) -> Option<String> {
    use xdr::ShapePropertiesChoice;
    match sp.shape_properties_choice1.as_ref()? {
        ShapePropertiesChoice::APrstGeom(g) => {
            // ooxmlsdk's preset enum has ~200 variants — Debug derives
            // the Pascal-case variant name (e.g. `Rect`, `RoundRect`,
            // `LeftArrow`). Lowercase the first char to match OOXML's
            // camelCase token form.
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
        // gradFill / pattFill / blipFill / grpFill / noFill: dropped.
        // gradFill could be solved via center-stop fallback but v0
        // just falls through to no fill (matches the "rect + fill +
        // text" remit in PARITY.md).
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
            // Reuse the chart color resolver via Debug repr. The
            // SchemeColor's variant Debug name is `Accent1` etc.,
            // which `theme_scheme_color` already handles.
            let dbg = format!("{:?}", c);
            crate::chart_colors::theme_scheme_color(&dbg, theme)
                .map(|base| crate::chart_colors::apply_color_modifiers(&base, &dbg))
        }
        SolidFillChoice::APrstClr(c) => {
            // PresetColorValues — use Debug variant for the well-known
            // English names. We resolve via a tiny table; unknowns
            // return None.
            let dbg = format!("{:?}", c.val);
            preset_color_hex(&dbg).map(|s| s.to_string())
        }
        SolidFillChoice::ASysClr(c) => {
            let last: Option<&str> = c.last_color.as_deref();
            last.map(|s| format!("#{}", s))
        }
        // scrgb / hsl: deferred (rare in shape fills).
        _ => None,
    }
}

/// Minimal English preset-color table. Covers what Excel's UI exposes
/// (the "Standard Colors" row) plus the named colors that appear in
/// the fallback markup of chartEx alternateContent blocks.
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

/// Read `<a:bodyPr lIns/tIns/rIns/bIns/>` insets in EMU. Returns
/// `None` when *all four* attrs are absent (so the renderer can
/// apply the DrawingML defaults wholesale). When at least one is
/// present, missing slots are filled with their respective default
/// (91440 / 45720 / 91440 / 45720 EMU per ECMA-376 §21.1.2.1.1).
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
    // `<a:bodyPr wrap="none"|"square"/>`. Default (attr absent) is
    // `square` per ECMA-376 §21.1.2.1.1 bodyPr — we surface `None` for that
    // default and let the renderer decide. We only need to flag the
    // explicit `none` case (no-wrap, run-on long lines).
    // `bp.wrap` is `Option<TextWrappingValues>`. Absent attr ⇒ outer
    // `None` (Debug "None") — we just return None (use renderer
    // default). Explicit `none` ⇒ `Some(None_)` (Debug "Some(None_)");
    // explicit `square` ⇒ `Some(Square)`.
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
    // `Option<TextAnchoringTypeValues>` Debug e.g. `Some(Center)`.
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
    // ECMA-376 alignment tokens: `l`/`ctr`/`r`/`just`/`justLow`/`dist`/`thaiDist`.
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
        // OOXML stores size in 1/100pt; TextRun.size is points.
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
        // Any `strike` attr (sng/dbl) → on. Renderer doesn't yet
        // distinguish single vs double strike for shapes.
        tr.strike = true;
    }
    // Color: `<a:solidFill>` inside the run properties.
    if let Some(a::RunPropertiesChoice::ASolidFill(sf)) = rp.run_properties_choice1.as_ref() {
        if let Some(hex) = resolve_solid_fill(sf, theme) {
            // Strip leading '#' — the schema Color::rgb is 6-char hex.
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
    // Latin font face.
    if let Some(latin) = rp.a_latin.as_ref() {
        let tf: &str = latin.typeface.as_deref().unwrap_or("");
        if !tf.is_empty() && !tf.starts_with('+') {
            // `+mn-lt` / `+mj-lt` are theme references; resolve to the
            // workbook's minor/major font when possible.
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
