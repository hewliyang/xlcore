//! Sparkline extraction.
//!
//! Sparklines live under the worksheet's `<extLst>` in the Office 2010
//! `x14` namespace (ext URI `{05C60535-1F16-4fd2-B633-F4F36F0B64E0}`).
//! Each `<x14:sparklineGroup>` carries shared chrome (type, axis colors,
//! marker toggles) plus N `<x14:sparkline>` children, where each
//! sparkline is anchored at one cell (`<xne:sqref>`) and pulls its
//! data from a range (`<xne:f>`). Values are resolved later in
//! `lib.rs::resolve_sparkline_refs` once every sheet is in hand.

use crate::schema::{Sparkline, SparklineGroup};
use ooxmlsdk::schemas::schemas_microsoft_com_office_spreadsheetml_2009_9_main as x14;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as xspread;
use xlcore_io::parse_a1;
#[allow(unused)]
use xspread::WorksheetExtensionChoice;

pub fn extract(ws: &xspread::Worksheet) -> Vec<SparklineGroup> {
    let Some(ext_lst) = &ws.x_ext_lst else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for ext in &ext_lst.x_ext {
        let Some(choice) = &ext.worksheet_extension_choice else {
            continue;
        };
        let xspread::WorksheetExtensionChoice::X14SparklineGroups(groups) = choice else {
            continue;
        };
        for g in &groups.x14_sparkline_group {
            out.push(extract_group(g));
        }
    }
    out
}

fn extract_group(g: &x14::SparklineGroup) -> SparklineGroup {
    let spark_type = match g.r#type {
        Some(x14::SparklineTypeValues::Line) | None => "line",
        Some(x14::SparklineTypeValues::Column) => "column",
        Some(x14::SparklineTypeValues::Stacked) => "stacked",
    }
    .to_string();

    let axis_kind = |v: Option<x14::SparklineAxisMinMaxValues>| {
        match v {
            Some(x14::SparklineAxisMinMaxValues::Individual) | None => "individual",
            Some(x14::SparklineAxisMinMaxValues::Group) => "group",
            Some(x14::SparklineAxisMinMaxValues::Custom) => "custom",
        }
        .to_string()
    };

    let display_empty = match g.display_empty_cells_as {
        Some(x14::DisplayBlanksAsValues::Gap) | None => "gap",
        Some(x14::DisplayBlanksAsValues::Zero) => "zero",
        Some(x14::DisplayBlanksAsValues::Span) => "span",
    }
    .to_string();

    let mut sparklines = Vec::new();
    for sp in &g.sparklines.x14_sparkline {
        // sqref is "A1" or sometimes a range; we anchor on the
        // top-left cell. Empty sqref = drop.
        let sqref = sp.reference_sequence.as_str().trim();
        if sqref.is_empty() {
            continue;
        }
        // Take first whitespace-separated token, then first ":" half.
        let first_token = sqref.split_whitespace().next().unwrap_or("");
        let first_cell = first_token.split(':').next().unwrap_or("");
        let Some((r, c)) = parse_a1(first_cell) else {
            continue;
        };
        sparklines.push(Sparkline {
            r,
            c,
            formula: sp.formula.as_ref().map(|s| s.as_str().to_string()),
            values: Vec::new(), // resolved post-pass in lib.rs
        });
    }

    SparklineGroup {
        spark_type,
        line_weight: g.line_weight.map(|v| v as f32).unwrap_or(0.75),
        markers: g.markers.unwrap_or(false),
        high: g.high.unwrap_or(false),
        low: g.low.unwrap_or(false),
        first: g.first.unwrap_or(false),
        last: g.last.unwrap_or(false),
        negative: g.negative.unwrap_or(false),
        display_x_axis: g.display_x_axis.unwrap_or(false),
        right_to_left: g.right_to_left.unwrap_or(false),
        display_empty_cells_as: display_empty,
        min_axis_type: axis_kind(g.min_axis_type),
        max_axis_type: axis_kind(g.max_axis_type),
        manual_min: g.manual_min,
        manual_max: g.manual_max,
        group_min: None, // resolved post-pass
        group_max: None,
        color_series: g
            .series_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        color_negative: g
            .negative_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        color_axis: g
            .axis_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        color_markers: g
            .markers_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        color_first: g
            .first_marker_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        color_last: g
            .last_marker_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        color_high: g
            .high_marker_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        color_low: g
            .low_marker_color
            .as_ref()
            .and_then(|c| color_hex(c.rgb.as_deref())),
        sparklines,
    }
}

/// Strip the leading alpha byte from an OOXML `aRRGGBB` hex string.
/// Theme/indexed/auto color resolution is intentionally skipped here:
/// Excel writes literal RGB on sparkline color elements in the vast
/// majority of files, and the renderer has a sensible accent fallback.
fn color_hex(rgb: Option<&str>) -> Option<String> {
    let s = rgb?;
    let trimmed = s.trim_start_matches('#');
    let hex = if trimmed.len() == 8 {
        &trimmed[2..]
    } else {
        trimmed
    };
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(hex.to_ascii_uppercase())
    } else {
        None
    }
}
