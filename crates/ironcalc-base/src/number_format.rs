use crate::{
    formatter::{self, format::Formatted},
    locale::get_locale,
    types::NumFmt,
};

// ECMA-376 Part 1 §18.8.30 "All Languages" built-in number formats.
// IDs 5–8, 23–36, 41–44, 50–58 are implementation-defined per the spec
// and intentionally omitted here; they must be carried as explicit
// `<numFmt>` entries on the styles part.
const BUILT_IN_FORMATS: &[(i32, &str)] = &[
    (0, "general"),
    (1, "0"),
    (2, "0.00"),
    (3, "#,##0"),
    (4, "#,##0.00"),
    (9, "0%"),
    (10, "0.00%"),
    (11, "0.00E+00"),
    (12, "# ?/?"),
    (13, "# ??/??"),
    (14, "mm-dd-yy"),
    (15, "d-mmm-yy"),
    (16, "d-mmm"),
    (17, "mmm-yy"),
    (18, "h:mm AM/PM"),
    (19, "h:mm:ss AM/PM"),
    (20, "h:mm"),
    (21, "h:mm:ss"),
    (22, "m/d/yy h:mm"),
    (37, "#,##0 ;(#,##0)"),
    (38, "#,##0 ;[Red](#,##0)"),
    (39, "#,##0.00;(#,##0.00)"),
    (40, "#,##0.00;[Red](#,##0.00)"),
    (45, "mm:ss"),
    (46, "[h]:mm:ss"),
    (47, "mmss.0"),
    (48, "##0.0E+0"),
    (49, "@"),
];

const CUSTOM_NUM_FMT_START: i32 = 164;

pub fn get_default_num_fmt_id(num_fmt: &str) -> Option<i32> {
    for (id, code) in BUILT_IN_FORMATS {
        if *code == num_fmt {
            return Some(*id);
        }
    }
    if num_fmt.eq_ignore_ascii_case("general") {
        return Some(0);
    }
    None
}

pub fn get_num_fmt(num_fmt_id: i32, num_fmts: &[NumFmt]) -> String {
    for num_fmt in num_fmts {
        if num_fmt.num_fmt_id == num_fmt_id {
            return num_fmt.format_code.clone();
        }
    }
    for (id, code) in BUILT_IN_FORMATS {
        if *id == num_fmt_id {
            return (*code).to_string();
        }
    }
    "general".to_string()
}

pub fn get_new_num_fmt_index(num_fmts: &[NumFmt]) -> i32 {
    let mut index = CUSTOM_NUM_FMT_START;
    loop {
        if num_fmts.iter().all(|nf| nf.num_fmt_id != index) {
            return index;
        }
        index += 1;
    }
}

pub fn to_precision(value: f64, precision: usize) -> f64 {
    if value.is_infinite() || value.is_nan() {
        return value;
    }
    to_precision_str(value, precision)
        .parse::<f64>()
        .unwrap_or({
            // TODO: do this in a way that does not require a possible error
            0.0
        })
}

/// It rounds a `f64` with `p` significant figures:
/// ```
///     use ironcalc_base::number_format;
///     assert_eq!(number_format::to_precision(0.1+0.2, 15), 0.3);
///     assert_eq!(number_format::to_excel_precision_str(0.1+0.2), "0.3");
/// ```
/// This intends to be equivalent to the js: `${parseFloat(value.toPrecision(precision)})`
/// See ([ecma](https://tc39.es/ecma262/#sec-number.prototype.toprecision)).
pub fn to_excel_precision_str(value: f64) -> String {
    to_precision_str(value, 15)
}

pub fn to_excel_precision(value: f64, precision: usize) -> f64 {
    if !value.is_finite() {
        return value;
    }

    let s = format!("{:.*e}", precision.saturating_sub(1), value);
    s.parse::<f64>().unwrap_or(value)
}

pub fn to_precision_str(value: f64, precision: usize) -> String {
    if !value.is_finite() {
        if value.is_infinite() {
            return "inf".to_string();
        } else {
            return "NaN".to_string();
        }
    }

    let s = format!("{:.*e}", precision.saturating_sub(1), value);
    let parsed = s.parse::<f64>().unwrap_or(value);

    // I would love to use the std library. There is not a speed concern here
    // problem is it doesn't do the right thing
    // Also ryu is my favorite _modern_ algorithm
    let mut buffer = ryu::Buffer::new();
    let text = buffer.format(parsed);
    // The above algorithm converts 2 to 2.0 regrettably
    if let Some(stripped) = text.strip_suffix(".0") {
        return stripped.to_string();
    }
    text.to_string()
}

pub fn format_number(value: f64, format_code: &str, locale: &str) -> Formatted {
    let locale = match get_locale(locale) {
        Ok(l) => l,
        Err(_) => {
            return Formatted {
                text: "#ERROR!".to_owned(),
                color: None,
                error: Some("Invalid locale".to_string()),
            }
        }
    };
    formatter::format::format_number(value, format_code, locale)
}
