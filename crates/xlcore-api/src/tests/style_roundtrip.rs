use crate::*;

fn norm_color(c: &str) -> String {
    let c = c.strip_prefix('#').unwrap_or(c).to_uppercase();
    if c.len() == 8 && c.starts_with("FF") {
        c[2..].to_string()
    } else {
        c
    }
}

fn color_superset(input: &Option<String>, output: &Option<String>) -> bool {
    match input {
        None => true,
        Some(i) => output.as_deref().map(norm_color) == Some(norm_color(i)),
    }
}

fn line_superset(input: &BorderLinePatch, output: Option<&BorderLinePatch>) -> bool {
    let Some(output) = output else { return false };
    input.style == output.style && color_superset(&input.color, &output.color)
}

fn font_superset(input: &FontPatch, output: &FontPatch) {
    if input.name.is_some() {
        assert_eq!(input.name, output.name, "font.name");
    }
    if input.size.is_some() {
        assert_eq!(input.size, output.size, "font.size");
    }
    if input.bold.is_some() {
        assert_eq!(input.bold, output.bold, "font.bold");
    }
    if input.italic.is_some() {
        assert_eq!(input.italic, output.italic, "font.italic");
    }
    if input.underline.is_some() {
        assert_eq!(input.underline, output.underline, "font.underline");
    }
    if input.strike.is_some() {
        assert_eq!(input.strike, output.strike, "font.strike");
    }
    if input.color.is_some() {
        assert!(
            color_superset(&input.color, &output.color),
            "font.color {:?} vs {:?}",
            input.color,
            output.color
        );
    }
    if input.vert_align.is_some() {
        assert_eq!(input.vert_align, output.vert_align, "font.vertAlign");
    }
    if input.family.is_some() {
        assert_eq!(input.family, output.family, "font.family");
    }
    if input.scheme.is_some() {
        assert_eq!(input.scheme, output.scheme, "font.scheme");
    }
}

fn fill_superset(input: &FillPatch, output: &FillPatch) {
    if let Some(gin) = input.gradient.as_ref() {
        let gout = output.gradient.as_ref().expect("gradient read back");
        if gin.kind.is_some() {
            assert_eq!(gin.kind, gout.kind, "gradient.kind");
        }
        if gin.degree.is_some() {
            assert_eq!(gin.degree, gout.degree, "gradient.degree");
        }
        assert_eq!(gin.stops.len(), gout.stops.len(), "gradient.stops len");
        for (a, b) in gin.stops.iter().zip(&gout.stops) {
            assert_eq!(a.position, b.position, "gradient stop position");
            assert_eq!(
                norm_color(&a.color),
                norm_color(&b.color),
                "gradient stop color"
            );
        }
        return;
    }
    if let Some(color) = input.color.as_ref() {
        assert_eq!(
            output.pattern,
            Some(PatternType::Solid),
            "fill solid pattern"
        );
        assert!(
            color_superset(&Some(color.clone()), &output.foreground),
            "fill color {:?} vs foreground {:?}",
            color,
            output.foreground
        );
    }
    if input.pattern.is_some() {
        assert_eq!(input.pattern, output.pattern, "fill.pattern");
    }
    if input.foreground.is_some() {
        assert!(
            color_superset(&input.foreground, &output.foreground),
            "fill.foreground"
        );
    }
    if input.background.is_some() {
        assert!(
            color_superset(&input.background, &output.background),
            "fill.background"
        );
    }
}

fn border_superset(input: &BorderPatch, output: &BorderPatch) {
    if let Some(all) = input.all.as_ref() {
        for (name, side) in [
            ("left", output.left.as_ref()),
            ("right", output.right.as_ref()),
            ("top", output.top.as_ref()),
            ("bottom", output.bottom.as_ref()),
        ] {
            assert!(line_superset(all, side), "border.all -> {name}");
        }
    }
    for (name, inp, out) in [
        ("left", &input.left, &output.left),
        ("right", &input.right, &output.right),
        ("top", &input.top, &output.top),
        ("bottom", &input.bottom, &output.bottom),
        ("diagonal", &input.diagonal, &output.diagonal),
    ] {
        if let Some(line) = inp {
            assert!(line_superset(line, out.as_ref()), "border.{name}");
        }
    }
    if input.diagonal_up.is_some() {
        assert_eq!(input.diagonal_up, output.diagonal_up, "border.diagonalUp");
    }
    if input.diagonal_down.is_some() {
        assert_eq!(
            input.diagonal_down, output.diagonal_down,
            "border.diagonalDown"
        );
    }
}

fn alignment_superset(input: &AlignmentPatch, output: &AlignmentPatch) {
    if input.horizontal.is_some() {
        assert_eq!(input.horizontal, output.horizontal, "align.horizontal");
    }
    if input.vertical.is_some() {
        assert_eq!(input.vertical, output.vertical, "align.vertical");
    }
    if input.wrap.is_some() {
        assert_eq!(input.wrap, output.wrap, "align.wrap");
    }
    if input.indent.is_some() {
        assert_eq!(input.indent, output.indent, "align.indent");
    }
    if input.text_rotation.is_some() {
        assert_eq!(
            input.text_rotation, output.text_rotation,
            "align.textRotation"
        );
    }
    if input.shrink_to_fit.is_some() {
        assert_eq!(input.shrink_to_fit, output.shrink_to_fit, "align.shrink");
    }
    if input.justify_last_line.is_some() {
        assert_eq!(
            input.justify_last_line, output.justify_last_line,
            "align.justifyLastLine"
        );
    }
    if input.reading_order.is_some() {
        assert_eq!(
            input.reading_order, output.reading_order,
            "align.readingOrder"
        );
    }
}

fn assert_superset(input: &StylePatch, output: &StylePatch) {
    if let Some(font) = input.font.as_ref() {
        font_superset(font, output.font.as_ref().expect("font read back"));
    }
    if let Some(fill) = input.fill.as_ref() {
        fill_superset(fill, output.fill.as_ref().expect("fill read back"));
    }
    if let Some(border) = input.border.as_ref() {
        border_superset(border, output.border.as_ref().expect("border read back"));
    }
    if let Some(align) = input.alignment.as_ref() {
        alignment_superset(
            align,
            output.alignment.as_ref().expect("alignment read back"),
        );
    }
    if input.number_format.is_some() {
        assert_eq!(input.number_format, output.number_format, "number_format");
    }
    if let Some(prot) = input.protection.as_ref() {
        let out = output.protection.as_ref().expect("protection read back");
        if prot.locked.is_some() {
            assert_eq!(prot.locked, out.locked, "protection.locked");
        }
        if prot.hidden.is_some() {
            assert_eq!(prot.hidden, out.hidden, "protection.hidden");
        }
    }
}

fn font(f: FontPatch) -> StylePatch {
    StylePatch {
        font: Some(f),
        ..Default::default()
    }
}

fn fill(f: FillPatch) -> StylePatch {
    StylePatch {
        fill: Some(f),
        ..Default::default()
    }
}

fn border(b: BorderPatch) -> StylePatch {
    StylePatch {
        border: Some(b),
        ..Default::default()
    }
}

fn align(a: AlignmentPatch) -> StylePatch {
    StylePatch {
        alignment: Some(a),
        ..Default::default()
    }
}

fn line(style: BorderLineStyle, color: &str) -> BorderLinePatch {
    BorderLinePatch {
        style,
        color: Some(color.to_string()),
    }
}

fn corpus() -> Vec<StylePatch> {
    vec![
        font(FontPatch {
            name: Some("Calibri".into()),
            size: Some(11.0),
            bold: Some(true),
            ..Default::default()
        }),
        font(FontPatch {
            italic: Some(true),
            strike: Some(true),
            underline: Some(UnderlinePatch::Single),
            ..Default::default()
        }),
        font(FontPatch {
            underline: Some(UnderlinePatch::Double),
            color: Some("#FF0000".into()),
            size: Some(18.0),
            ..Default::default()
        }),
        font(FontPatch {
            color: Some("FF00AA33".into()),
            vert_align: Some(VertAlign::Superscript),
            ..Default::default()
        }),
        font(FontPatch {
            vert_align: Some(VertAlign::Subscript),
            family: Some(2),
            scheme: Some(FontScheme::Minor),
            ..Default::default()
        }),
        font(FontPatch {
            scheme: Some(FontScheme::Major),
            ..Default::default()
        }),
        fill(FillPatch {
            color: Some("E2F0D9".into()),
            ..Default::default()
        }),
        fill(FillPatch {
            color: Some("#1F4E78".into()),
            ..Default::default()
        }),
        fill(FillPatch {
            pattern: Some(PatternType::DarkGrid),
            foreground: Some("FF0000".into()),
            background: Some("FFFFFF".into()),
            ..Default::default()
        }),
        fill(FillPatch {
            pattern: Some(PatternType::LightUp),
            foreground: Some("000000".into()),
            ..Default::default()
        }),
        fill(FillPatch {
            gradient: Some(GradientFillPatch {
                kind: Some(GradientType::Linear),
                degree: Some(90.0),
                stops: vec![
                    GradientStopPatch {
                        position: 0.0,
                        color: "FF0000".into(),
                    },
                    GradientStopPatch {
                        position: 1.0,
                        color: "0000FF".into(),
                    },
                ],
                ..Default::default()
            }),
            ..Default::default()
        }),
        border(BorderPatch {
            left: Some(line(BorderLineStyle::Thin, "000000")),
            ..Default::default()
        }),
        border(BorderPatch {
            right: Some(line(BorderLineStyle::Medium, "112233")),
            ..Default::default()
        }),
        border(BorderPatch {
            top: Some(line(BorderLineStyle::Dashed, "FF0000")),
            ..Default::default()
        }),
        border(BorderPatch {
            bottom: Some(line(BorderLineStyle::Double, "00FF00")),
            ..Default::default()
        }),
        border(BorderPatch {
            all: Some(line(BorderLineStyle::Thick, "0000FF")),
            ..Default::default()
        }),
        border(BorderPatch {
            diagonal: Some(line(BorderLineStyle::Thin, "FF0000")),
            diagonal_up: Some(true),
            diagonal_down: Some(true),
            ..Default::default()
        }),
        align(AlignmentPatch {
            horizontal: Some(HorizontalAlign::Center),
            vertical: Some(VerticalAlign::Top),
            wrap: Some(true),
            indent: Some(2),
            ..Default::default()
        }),
        align(AlignmentPatch {
            text_rotation: Some(45),
            ..Default::default()
        }),
        align(AlignmentPatch {
            text_rotation: Some(-30),
            ..Default::default()
        }),
        align(AlignmentPatch {
            shrink_to_fit: Some(true),
            justify_last_line: Some(true),
            reading_order: Some(ReadingOrder::RightToLeft),
            ..Default::default()
        }),
        align(AlignmentPatch {
            reading_order: Some(ReadingOrder::LeftToRight),
            ..Default::default()
        }),
        StylePatch {
            number_format: Some("0.00".into()),
            ..Default::default()
        },
        StylePatch {
            number_format: Some("#,##0.00".into()),
            ..Default::default()
        },
        StylePatch {
            number_format: Some("0%".into()),
            ..Default::default()
        },
        StylePatch {
            number_format: Some("0.000".into()),
            ..Default::default()
        },
        StylePatch {
            protection: Some(ProtectionPatch {
                locked: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        },
        StylePatch {
            protection: Some(ProtectionPatch {
                locked: Some(true),
                hidden: Some(true),
            }),
            ..Default::default()
        },
    ]
}

#[test]
fn style_patch_round_trips_across_corpus() {
    let mut wb = Workbook::new().unwrap();
    for (i, patch) in corpus().into_iter().enumerate() {
        let cell = format!("Sheet1!A{}", i + 1);
        wb.set_value(&cell, "x").unwrap();
        wb.set_style(&cell, patch.clone()).unwrap();
        let out = wb
            .get_cell(&cell)
            .unwrap()
            .style
            .unwrap_or_else(|| panic!("no style read back for {cell}: {patch:?}"));
        assert_superset(&patch, &out);
    }
}

#[test]
fn named_style_master_reads_back_through_cell() {
    let mut wb = Workbook::new().unwrap();
    let master = StylePatch {
        font: Some(FontPatch {
            bold: Some(true),
            color: Some("006100".into()),
            ..Default::default()
        }),
        fill: Some(FillPatch {
            color: Some("C6EFCE".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    wb.set_named_style(NamedStylePatch {
        name: "Good".into(),
        builtin_id: Some(26),
        style: master.clone(),
    })
    .unwrap();
    wb.set_value("Sheet1!A1", "x").unwrap();
    wb.set_style(
        "Sheet1!A1",
        StylePatch {
            named_style: Some("Good".into()),
            ..Default::default()
        },
    )
    .unwrap();

    let out = wb.get_cell("Sheet1!A1").unwrap().style.unwrap();
    assert_superset(&master, &out);
}

#[test]
fn none_border_side_clears_to_no_border() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "x").unwrap();
    wb.set_style(
        "Sheet1!A1",
        border(BorderPatch {
            left: Some(BorderLinePatch {
                style: BorderLineStyle::None,
                color: None,
            }),
            ..Default::default()
        }),
    )
    .unwrap();

    let out = wb.get_cell("Sheet1!A1").unwrap().style;
    assert!(
        out.as_ref().and_then(|s| s.border.as_ref()).is_none(),
        "None side leaves no border: {out:?}"
    );
}
