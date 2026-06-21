use crate::{
    calc_result::CalcResult,
    constants::{LAST_COLUMN, LAST_ROW},
    expressions::{
        parser::{ArrayNode, Node},
        token::Error,
        types::CellReferenceIndex,
    },
    formatter::format::{format_number, parse_formatted_number},
    model::Model,
    number_format::to_precision,
};

use super::{
    text_util::{substitute, text_after, text_before, Case},
    util::from_wildcard_to_regex,
};

fn split_on_any(text: &str, delimiters: &[String], case_insensitive: bool) -> Vec<String> {
    let mut result = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let hay: Vec<char> = if case_insensitive {
        text.to_lowercase().chars().collect()
    } else {
        chars.clone()
    };
    let needles: Vec<Vec<char>> = delimiters
        .iter()
        .map(|d| {
            if case_insensitive {
                d.to_lowercase().chars().collect()
            } else {
                d.chars().collect()
            }
        })
        .collect();
    let mut start = 0;
    let mut i = 0;
    while i < chars.len() {
        let mut matched = None;
        for needle in &needles {
            let n = needle.len();
            if n > 0 && i + n <= hay.len() && hay[i..i + n] == needle[..] {
                matched = Some(n);
                break;
            }
        }
        if let Some(n) = matched {
            result.push(chars[start..i].iter().collect());
            i += n;
            start = i;
        } else {
            i += 1;
        }
    }
    result.push(chars[start..].iter().collect());
    result
}

fn array_to_text_value(value: &CalcResult, strict: bool) -> String {
    match value {
        CalcResult::Number(f) => format!("{f}"),
        CalcResult::String(s) => {
            if strict {
                format!("\"{s}\"")
            } else {
                s.clone()
            }
        }
        CalcResult::Boolean(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        CalcResult::EmptyCell | CalcResult::EmptyArg => {
            if strict {
                "\"\"".to_string()
            } else {
                String::new()
            }
        }
        CalcResult::Error { error, .. } => error.to_string(),
        CalcResult::Range { .. } | CalcResult::Array(_) | CalcResult::Lambda { .. } => String::new(),
    }
}

/// Finds the first instance of 'search_for' in text starting at char index start
fn find(search_for: &str, text: &str, start: usize) -> Option<i32> {
    let ch = text.chars();
    let mut byte_index = 0;
    for (char_index, c) in ch.enumerate() {
        if char_index + 1 >= start && text[byte_index..].starts_with(search_for) {
            return Some((char_index + 1) as i32);
        }
        byte_index += c.len_utf8();
    }
    None
}

/// You can use the wildcard characters — the question mark (?) and asterisk (*) — in the find_text argument.
/// * A question mark matches any single character.
/// * An asterisk matches any sequence of characters.
/// * If you want to find an actual question mark or asterisk, type a tilde (~) before the character.
fn search(search_for: &str, text: &str, start: usize) -> Option<i32> {
    let re = match from_wildcard_to_regex(search_for, false) {
        Ok(r) => r,
        Err(_) => return None,
    };

    let ch = text.chars();
    let mut byte_index = 0;
    for (char_index, c) in ch.enumerate() {
        if char_index + 1 >= start {
            if let Some(m) = re.find(&text[byte_index..]) {
                return Some((text[0..(m.start() + byte_index)].chars().count() as i32) + 1);
            } else {
                return None;
            }
        }
        byte_index += c.len_utf8();
    }
    None
}

const CP1252_HIGH: [Option<char>; 32] = [
    Some('\u{20AC}'),
    None,
    Some('\u{201A}'),
    Some('\u{0192}'),
    Some('\u{201E}'),
    Some('\u{2026}'),
    Some('\u{2020}'),
    Some('\u{2021}'),
    Some('\u{02C6}'),
    Some('\u{2030}'),
    Some('\u{0160}'),
    Some('\u{2039}'),
    Some('\u{0152}'),
    None,
    Some('\u{017D}'),
    None,
    None,
    Some('\u{2018}'),
    Some('\u{2019}'),
    Some('\u{201C}'),
    Some('\u{201D}'),
    Some('\u{2022}'),
    Some('\u{2013}'),
    Some('\u{2014}'),
    Some('\u{02DC}'),
    Some('\u{2122}'),
    Some('\u{0161}'),
    Some('\u{203A}'),
    Some('\u{0153}'),
    None,
    Some('\u{017E}'),
    Some('\u{0178}'),
];

fn cp1252_to_char(code: u32) -> Option<char> {
    if (128..=159).contains(&code) {
        CP1252_HIGH[(code - 128) as usize]
    } else if code <= 255 {
        char::from_u32(code)
    } else {
        None
    }
}

fn char_to_cp1252(c: char) -> u32 {
    let code = c as u32;
    if code <= 255 {
        return code;
    }
    for (index, mapped) in CP1252_HIGH.iter().enumerate() {
        if *mapped == Some(c) {
            return 128 + index as u32;
        }
    }
    63
}

fn thai_digit(d: u8) -> &'static str {
    match d {
        0 => "ศูนย์",
        1 => "หนึ่ง",
        2 => "สอง",
        3 => "สาม",
        4 => "สี่",
        5 => "ห้า",
        6 => "หก",
        7 => "เจ็ด",
        8 => "แปด",
        _ => "เก้า",
    }
}

fn thai_read_group(s: &str) -> String {
    let positions = ["", "สิบ", "ร้อย", "พัน", "หมื่น", "แสน"];
    let digits: Vec<u8> = s.bytes().map(|b| b - b'0').collect();
    let mut start = 0;
    while start < digits.len() && digits[start] == 0 {
        start += 1;
    }
    let digits = &digits[start..];
    let len = digits.len();
    let mut result = String::new();
    for (i, &d) in digits.iter().enumerate() {
        if d == 0 {
            continue;
        }
        let pos = len - 1 - i;
        if pos == 0 && d == 1 && len > 1 {
            result.push_str("เอ็ด");
        } else if pos == 1 && d == 2 {
            result.push_str("ยี่สิบ");
        } else if pos == 1 && d == 1 {
            result.push_str("สิบ");
        } else {
            result.push_str(thai_digit(d));
            result.push_str(positions[pos]);
        }
    }
    result
}

fn thai_read_integer(s: &str) -> String {
    let trimmed = s.trim_start_matches('0');
    if trimmed.is_empty() {
        return "ศูนย์".to_string();
    }
    if trimmed.len() <= 6 {
        return thai_read_group(trimmed);
    }
    let split = trimmed.len() - 6;
    format!(
        "{}ล้าน{}",
        thai_read_integer(&trimmed[..split]),
        thai_read_group(&trimmed[split..])
    )
}

fn baht_text(number: f64) -> String {
    let total = (number.abs() * 100.0).round();
    if !total.is_finite() {
        return String::new();
    }
    let total = total as u128;
    let integer = total / 100;
    let satang = (total % 100) as u8;
    let mut result = String::new();
    if number < 0.0 && total != 0 {
        result.push_str("ลบ");
    }
    result.push_str(&thai_read_integer(&integer.to_string()));
    result.push_str("บาท");
    if satang == 0 {
        result.push_str("ถ้วน");
    } else {
        result.push_str(&thai_read_group(&format!("{satang:02}")));
        result.push_str("สตางค์");
    }
    result
}

fn group_integer(int_part: &str, group: &str) -> String {
    let bytes: Vec<char> = int_part.chars().collect();
    let mut result = String::new();
    let len = bytes.len();
    for (index, ch) in bytes.iter().enumerate() {
        if index != 0 && (len - index) % 3 == 0 {
            result.push_str(group);
        }
        result.push(*ch);
    }
    result
}

fn format_fixed_magnitude(
    abs_value: f64,
    decimals: i32,
    commas: bool,
    group: &str,
    decimal: &str,
) -> String {
    let display_decimals = decimals.max(0) as usize;
    let s = format!("{abs_value:.display_decimals$}");
    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => (i.to_string(), Some(f.to_string())),
        None => (s, None),
    };
    let int_part = if commas {
        group_integer(&int_part, group)
    } else {
        int_part
    };
    match frac_part {
        Some(f) => format!("{int_part}{decimal}{f}"),
        None => int_part,
    }
}

fn round_to_decimals(value: f64, decimals: i32) -> f64 {
    let factor = 10f64.powi(decimals);
    (value * factor).round() / factor
}

impl<'a> Model<'a> {
    pub(crate) fn fn_concat(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let mut result = "".to_string();
        for arg in args {
            match self.evaluate_node_in_context(arg, cell) {
                CalcResult::String(value) => result = format!("{result}{value}"),
                CalcResult::Number(value) => result = format!("{result}{value}"),
                CalcResult::EmptyCell | CalcResult::EmptyArg => {}
                CalcResult::Boolean(value) => {
                    if value {
                        result = format!("{result}TRUE");
                    } else {
                        result = format!("{result}FALSE");
                    }
                }
                error @ CalcResult::Error { .. } => return error,
                CalcResult::Range { left, right } => {
                    if left.sheet != right.sheet {
                        return CalcResult::new_error(
                            Error::VALUE,
                            cell,
                            "Ranges are in different sheets".to_string(),
                        );
                    }
                    for row in left.row..(right.row + 1) {
                        for column in left.column..(right.column + 1) {
                            match self.evaluate_cell(CellReferenceIndex {
                                sheet: left.sheet,
                                row,
                                column,
                            }) {
                                CalcResult::String(value) => {
                                    result = format!("{result}{value}");
                                }
                                CalcResult::Number(value) => result = format!("{result}{value}"),
                                CalcResult::Boolean(value) => {
                                    if value {
                                        result = format!("{result}TRUE");
                                    } else {
                                        result = format!("{result}FALSE");
                                    }
                                }
                                error @ CalcResult::Error { .. } => return error,
                                CalcResult::EmptyCell | CalcResult::EmptyArg => {}
                                CalcResult::Range { .. } => {}
                                CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                                    return CalcResult::Error {
                                        error: Error::NIMPL,
                                        origin: cell,
                                        message: "Arrays not supported yet".to_string(),
                                    }
                                }
                            }
                        }
                    }
                }
                CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Arrays not supported yet".to_string(),
                    }
                }
            };
        }
        CalcResult::String(result)
    }
    pub(crate) fn fn_text(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() == 2 {
            let value = match self.evaluate_node_in_context(&args[0], cell) {
                CalcResult::Number(f) => f,
                CalcResult::String(s) => {
                    return CalcResult::String(s);
                }
                CalcResult::Boolean(b) => {
                    return CalcResult::Boolean(b);
                }
                error @ CalcResult::Error { .. } => return error,
                CalcResult::Range { .. } => {
                    // Implicit Intersection not implemented
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Implicit Intersection not implemented".to_string(),
                    };
                }
                CalcResult::EmptyCell | CalcResult::EmptyArg => 0.0,
                CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Arrays not supported yet".to_string(),
                    }
                }
            };
            let format_code = match self.get_string(&args[1], cell) {
                Ok(s) => s,
                Err(s) => return s,
            };
            let d = format_number(value, &format_code, self.locale);
            if let Some(_e) = d.error {
                return CalcResult::Error {
                    error: Error::VALUE,
                    origin: cell,
                    message: "Invalid format code".to_string(),
                };
            }
            CalcResult::String(d.text)
        } else {
            CalcResult::new_args_number_error(cell)
        }
    }

    /// FIND(find_text, within_text, [start_num])
    ///  * FIND and FINDB are case sensitive and don't allow wildcard characters.
    ///  * If find_text is "" (empty text), FIND matches the first character in the search string (that is, the character numbered start_num or 1).
    ///  * Find_text cannot contain any wildcard characters.
    ///  * If find_text does not appear in within_text, FIND and FINDB return the #VALUE! error value.
    ///  * If start_num is not greater than zero, FIND and FINDB return the #VALUE! error value.
    ///  * If start_num is greater than the length of within_text, FIND and FINDB return the #VALUE! error value.
    ///    NB: FINDB is not implemented. It is the same as FIND function unless locale is a DBCS (Double Byte Character Set)
    pub(crate) fn fn_find(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 2 || args.len() > 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let find_text = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(s) => return s,
        };
        let within_text = match self.get_string(&args[1], cell) {
            Ok(s) => s,
            Err(s) => return s,
        };
        let start_num = if args.len() == 3 {
            match self.get_number(&args[2], cell) {
                Ok(s) => s.floor(),
                Err(s) => return s,
            }
        } else {
            1.0
        };

        if start_num < 1.0 {
            return CalcResult::Error {
                error: Error::VALUE,
                origin: cell,
                message: "Start num must be >= 1".to_string(),
            };
        }
        let start_num = start_num as usize;

        if start_num > within_text.len() {
            return CalcResult::Error {
                error: Error::VALUE,
                origin: cell,
                message: "Start num greater than length".to_string(),
            };
        }
        if let Some(s) = find(&find_text, &within_text, start_num) {
            CalcResult::Number(s as f64)
        } else {
            CalcResult::Error {
                error: Error::VALUE,
                origin: cell,
                message: "Text not found".to_string(),
            }
        }
    }

    /// Same API as FIND but:
    ///  * Allows wildcards
    ///  * It is case insensitive
    ///    SEARCH(find_text, within_text, [start_num])
    pub(crate) fn fn_search(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 2 || args.len() > 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let find_text = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(s) => return s,
        };
        let within_text = match self.get_string(&args[1], cell) {
            Ok(s) => s,
            Err(s) => return s,
        };
        let start_num = if args.len() == 3 {
            match self.get_number(&args[2], cell) {
                Ok(s) => s.floor(),
                Err(s) => return s,
            }
        } else {
            1.0
        };

        if start_num < 1.0 {
            return CalcResult::Error {
                error: Error::VALUE,
                origin: cell,
                message: "Start num must be >= 1".to_string(),
            };
        }
        let start_num = start_num as usize;

        if start_num > within_text.len() {
            return CalcResult::Error {
                error: Error::VALUE,
                origin: cell,
                message: "Start num greater than length".to_string(),
            };
        }
        // SEARCH is case insensitive
        if let Some(s) = search(
            &find_text.to_lowercase(),
            &within_text.to_lowercase(),
            start_num,
        ) {
            CalcResult::Number(s as f64)
        } else {
            CalcResult::Error {
                error: Error::VALUE,
                origin: cell,
                message: "Text not found".to_string(),
            }
        }
    }

    // LEN, LEFT, RIGHT, MID, LOWER, UPPER, TRIM
    pub(crate) fn fn_len(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() == 1 {
            let s = match self.evaluate_node_in_context(&args[0], cell) {
                CalcResult::Number(v) => format!("{v}"),
                CalcResult::String(v) => v,
                CalcResult::Boolean(b) => {
                    if b {
                        "TRUE".to_string()
                    } else {
                        "FALSE".to_string()
                    }
                }
                error @ CalcResult::Error { .. } => return error,
                CalcResult::Range { .. } => {
                    // Implicit Intersection not implemented
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Implicit Intersection not implemented".to_string(),
                    };
                }
                CalcResult::EmptyCell | CalcResult::EmptyArg => "".to_string(),
                CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Arrays not supported yet".to_string(),
                    }
                }
            };
            return CalcResult::Number(s.chars().count() as f64);
        }
        CalcResult::new_args_number_error(cell)
    }

    pub(crate) fn fn_trim(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() == 1 {
            let s = match self.evaluate_node_in_context(&args[0], cell) {
                CalcResult::Number(v) => format!("{v}"),
                CalcResult::String(v) => v,
                CalcResult::Boolean(b) => {
                    if b {
                        "TRUE".to_string()
                    } else {
                        "FALSE".to_string()
                    }
                }
                error @ CalcResult::Error { .. } => return error,
                CalcResult::Range { .. } => {
                    // Implicit Intersection not implemented
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Implicit Intersection not implemented".to_string(),
                    };
                }
                CalcResult::EmptyCell | CalcResult::EmptyArg => "".to_string(),
                CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Arrays not supported yet".to_string(),
                    }
                }
            };
            return CalcResult::String(s.trim().to_owned());
        }
        CalcResult::new_args_number_error(cell)
    }

    pub(crate) fn fn_lower(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() == 1 {
            let s = match self.evaluate_node_in_context(&args[0], cell) {
                CalcResult::Number(v) => format!("{v}"),
                CalcResult::String(v) => v,
                CalcResult::Boolean(b) => {
                    if b {
                        "TRUE".to_string()
                    } else {
                        "FALSE".to_string()
                    }
                }
                error @ CalcResult::Error { .. } => return error,
                CalcResult::Range { .. } => {
                    // Implicit Intersection not implemented
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Implicit Intersection not implemented".to_string(),
                    };
                }
                CalcResult::EmptyCell | CalcResult::EmptyArg => "".to_string(),
                CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Arrays not supported yet".to_string(),
                    }
                }
            };
            return CalcResult::String(s.to_lowercase());
        }
        CalcResult::new_args_number_error(cell)
    }

    pub(crate) fn fn_unicode(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() == 1 {
            let s = match self.evaluate_node_in_context(&args[0], cell) {
                CalcResult::Number(v) => format!("{v}"),
                CalcResult::String(v) => v,
                CalcResult::Boolean(b) => {
                    if b {
                        "TRUE".to_string()
                    } else {
                        "FALSE".to_string()
                    }
                }
                error @ CalcResult::Error { .. } => return error,
                CalcResult::Range { .. } => {
                    // Implicit Intersection not implemented
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Implicit Intersection not implemented".to_string(),
                    };
                }
                CalcResult::EmptyCell | CalcResult::EmptyArg => {
                    return CalcResult::Error {
                        error: Error::VALUE,
                        origin: cell,
                        message: "Empty cell".to_string(),
                    }
                }
                CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Arrays not supported yet".to_string(),
                    }
                }
            };

            match s.chars().next() {
                Some(c) => {
                    let unicode_number = c as u32;
                    return CalcResult::Number(unicode_number as f64);
                }
                None => {
                    return CalcResult::Error {
                        error: Error::VALUE,
                        origin: cell,
                        message: "Empty cell".to_string(),
                    };
                }
            }
        }
        CalcResult::new_args_number_error(cell)
    }

    pub(crate) fn fn_unichar(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        let value = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(error) => return error,
        };
        let code = value.trunc();
        if code < 1.0 || code > 1_114_111.0 {
            return CalcResult::new_error(Error::VALUE, cell, "Number out of range".to_string());
        }
        match char::from_u32(code as u32) {
            Some(c) => CalcResult::String(c.to_string()),
            None => CalcResult::new_error(Error::VALUE, cell, "Invalid code point".to_string()),
        }
    }

    pub(crate) fn fn_numbervalue(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let text = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let decimal_sep = if args.len() >= 2 {
            match self.get_string(&args[1], cell) {
                Ok(s) => s.chars().next().unwrap_or('.'),
                Err(error) => return error,
            }
        } else {
            '.'
        };
        let group_sep = if args.len() >= 3 {
            match self.get_string(&args[2], cell) {
                Ok(s) => s.chars().next().unwrap_or(','),
                Err(error) => return error,
            }
        } else {
            ','
        };
        let cleaned: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        if cleaned.is_empty() {
            return CalcResult::Number(0.0);
        }
        let mut percent_count = 0usize;
        let trimmed = {
            let mut s = cleaned.as_str();
            while let Some(rest) = s.strip_suffix('%') {
                percent_count += 1;
                s = rest;
            }
            s.to_string()
        };
        let mut normalized = String::new();
        let mut decimal_count = 0usize;
        for c in trimmed.chars() {
            if c == group_sep {
                continue;
            } else if c == decimal_sep {
                decimal_count += 1;
                normalized.push('.');
            } else {
                normalized.push(c);
            }
        }
        if decimal_count > 1 {
            return CalcResult::new_error(Error::VALUE, cell, "Invalid number".to_string());
        }
        match normalized.parse::<f64>() {
            Ok(v) => {
                let mut result = v;
                for _ in 0..percent_count {
                    result /= 100.0;
                }
                CalcResult::Number(result)
            }
            Err(_) => CalcResult::new_error(Error::VALUE, cell, "Invalid number".to_string()),
        }
    }

    pub(crate) fn fn_upper(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() == 1 {
            let s = match self.evaluate_node_in_context(&args[0], cell) {
                CalcResult::Number(v) => format!("{v}"),
                CalcResult::String(v) => v,
                CalcResult::Boolean(b) => {
                    if b {
                        "TRUE".to_string()
                    } else {
                        "FALSE".to_string()
                    }
                }
                error @ CalcResult::Error { .. } => return error,
                CalcResult::Range { .. } => {
                    // Implicit Intersection not implemented
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Implicit Intersection not implemented".to_string(),
                    };
                }
                CalcResult::EmptyCell | CalcResult::EmptyArg => "".to_string(),
                CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Arrays not supported yet".to_string(),
                    }
                }
            };
            return CalcResult::String(s.to_uppercase());
        }
        CalcResult::new_args_number_error(cell)
    }

    pub(crate) fn fn_left(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() > 2 || args.is_empty() {
            return CalcResult::new_args_number_error(cell);
        }
        let s = match self.evaluate_node_in_context(&args[0], cell) {
            CalcResult::Number(v) => format!("{v}"),
            CalcResult::String(v) => v,
            CalcResult::Boolean(b) => {
                if b {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            error @ CalcResult::Error { .. } => return error,
            CalcResult::Range { .. } => {
                // Implicit Intersection not implemented
                return CalcResult::Error {
                    error: Error::NIMPL,
                    origin: cell,
                    message: "Implicit Intersection not implemented".to_string(),
                };
            }
            CalcResult::EmptyCell | CalcResult::EmptyArg => "".to_string(),
            CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                return CalcResult::Error {
                    error: Error::NIMPL,
                    origin: cell,
                    message: "Arrays not supported yet".to_string(),
                }
            }
        };
        let num_chars = if args.len() == 2 {
            match self.evaluate_node_in_context(&args[1], cell) {
                CalcResult::Number(v) => {
                    if v < 0.0 {
                        return CalcResult::Error {
                            error: Error::VALUE,
                            origin: cell,
                            message: "Number must be >= 0".to_string(),
                        };
                    }
                    v.floor() as usize
                }
                CalcResult::Boolean(_) | CalcResult::String(_) => {
                    return CalcResult::Error {
                        error: Error::VALUE,
                        origin: cell,
                        message: "Expecting number".to_string(),
                    };
                }
                error @ CalcResult::Error { .. } => return error,
                CalcResult::Range { .. } => {
                    // Implicit Intersection not implemented
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Implicit Intersection not implemented".to_string(),
                    };
                }
                CalcResult::EmptyCell | CalcResult::EmptyArg => 0,
                CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Arrays not supported yet".to_string(),
                    }
                }
            }
        } else {
            1
        };
        let mut result = "".to_string();
        for (index, ch) in s.chars().enumerate() {
            if index >= num_chars {
                break;
            }
            result.push(ch);
        }
        CalcResult::String(result)
    }

    pub(crate) fn fn_right(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() > 2 || args.is_empty() {
            return CalcResult::new_args_number_error(cell);
        }
        let s = match self.evaluate_node_in_context(&args[0], cell) {
            CalcResult::Number(v) => format!("{v}"),
            CalcResult::String(v) => v,
            CalcResult::Boolean(b) => {
                if b {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            error @ CalcResult::Error { .. } => return error,
            CalcResult::Range { .. } => {
                // Implicit Intersection not implemented
                return CalcResult::Error {
                    error: Error::NIMPL,
                    origin: cell,
                    message: "Implicit Intersection not implemented".to_string(),
                };
            }
            CalcResult::EmptyCell | CalcResult::EmptyArg => "".to_string(),
            CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                return CalcResult::Error {
                    error: Error::NIMPL,
                    origin: cell,
                    message: "Arrays not supported yet".to_string(),
                }
            }
        };
        let num_chars = if args.len() == 2 {
            match self.evaluate_node_in_context(&args[1], cell) {
                CalcResult::Number(v) => {
                    if v < 0.0 {
                        return CalcResult::Error {
                            error: Error::VALUE,
                            origin: cell,
                            message: "Number must be >= 0".to_string(),
                        };
                    }
                    v.floor() as usize
                }
                CalcResult::Boolean(_) | CalcResult::String(_) => {
                    return CalcResult::Error {
                        error: Error::VALUE,
                        origin: cell,
                        message: "Expecting number".to_string(),
                    };
                }
                error @ CalcResult::Error { .. } => return error,
                CalcResult::Range { .. } => {
                    // Implicit Intersection not implemented
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Implicit Intersection not implemented".to_string(),
                    };
                }
                CalcResult::EmptyCell | CalcResult::EmptyArg => 0,
                CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Arrays not supported yet".to_string(),
                    }
                }
            }
        } else {
            1
        };
        let mut result = "".to_string();
        for (index, ch) in s.chars().rev().enumerate() {
            if index >= num_chars {
                break;
            }
            result.push(ch);
        }
        CalcResult::String(result.chars().rev().collect::<String>())
    }

    pub(crate) fn fn_mid(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let s = match self.evaluate_node_in_context(&args[0], cell) {
            CalcResult::Number(v) => format!("{v}"),
            CalcResult::String(v) => v,
            CalcResult::Boolean(b) => {
                if b {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            error @ CalcResult::Error { .. } => return error,
            CalcResult::Range { .. } => {
                // Implicit Intersection not implemented
                return CalcResult::Error {
                    error: Error::NIMPL,
                    origin: cell,
                    message: "Implicit Intersection not implemented".to_string(),
                };
            }
            CalcResult::EmptyCell | CalcResult::EmptyArg => "".to_string(),
            CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                return CalcResult::Error {
                    error: Error::NIMPL,
                    origin: cell,
                    message: "Arrays not supported yet".to_string(),
                }
            }
        };
        let start_num = match self.evaluate_node_in_context(&args[1], cell) {
            CalcResult::Number(v) => {
                if v < 1.0 {
                    return CalcResult::Error {
                        error: Error::VALUE,
                        origin: cell,
                        message: "Number must be >= 1".to_string(),
                    };
                }
                v.floor() as usize
            }
            error @ CalcResult::Error { .. } => return error,
            CalcResult::Range { .. } => {
                // Implicit Intersection not implemented
                return CalcResult::Error {
                    error: Error::NIMPL,
                    origin: cell,
                    message: "Implicit Intersection not implemented".to_string(),
                };
            }
            _ => {
                return CalcResult::Error {
                    error: Error::VALUE,
                    origin: cell,
                    message: "Expecting number".to_string(),
                };
            }
        };
        let num_chars = match self.evaluate_node_in_context(&args[2], cell) {
            CalcResult::Number(v) => {
                if v < 0.0 {
                    return CalcResult::Error {
                        error: Error::VALUE,
                        origin: cell,
                        message: "Number must be >= 0".to_string(),
                    };
                }
                v.floor() as usize
            }
            CalcResult::String(_) => {
                return CalcResult::Error {
                    error: Error::VALUE,
                    origin: cell,
                    message: "Expecting number".to_string(),
                };
            }
            CalcResult::Boolean(_) => {
                return CalcResult::Error {
                    error: Error::VALUE,
                    origin: cell,
                    message: "Expecting number".to_string(),
                }
            }
            error @ CalcResult::Error { .. } => return error,
            CalcResult::Range { .. } => {
                // Implicit Intersection not implemented
                return CalcResult::Error {
                    error: Error::NIMPL,
                    origin: cell,
                    message: "Implicit Intersection not implemented".to_string(),
                };
            }
            CalcResult::EmptyCell | CalcResult::EmptyArg => 0,
            CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                return CalcResult::Error {
                    error: Error::NIMPL,
                    origin: cell,
                    message: "Arrays not supported yet".to_string(),
                }
            }
        };
        let mut result = "".to_string();
        let mut count: usize = 0;
        for (index, ch) in s.chars().enumerate() {
            if count >= num_chars {
                break;
            }
            if index + 1 >= start_num {
                result.push(ch);
                count += 1;
            }
        }
        CalcResult::String(result)
    }

    // REPT(text, number_times)
    pub(crate) fn fn_rept(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let text = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let number_times = match self.get_number(&args[1], cell) {
            Ok(f) => f.floor() as i32,
            Err(s) => return s,
        };
        let text_len = text.len() as i32;

        // We normally don't follow Excel's sometimes archaic size's restrictions
        // But this might be a security issue
        if text_len * number_times > 32767 {
            return CalcResult::Error {
                error: Error::VALUE,
                origin: cell,
                message: "number times too high".to_string(),
            };
        }
        if number_times < 0 {
            return CalcResult::Error {
                error: Error::VALUE,
                origin: cell,
                message: "number times too high".to_string(),
            };
        }
        if number_times == 0 {
            return CalcResult::String("".to_string());
        }
        CalcResult::String(text.repeat(number_times as usize))
    }

    // TEXTAFTER(text, delimiter, [instance_num], [match_mode], [match_end], [if_not_found])
    pub(crate) fn fn_textafter(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(2..=6).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let text = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let delimiter = match self.get_string(&args[1], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let instance_num = if arg_count > 2 {
            match self.get_number(&args[2], cell) {
                Ok(f) => f.floor() as i32,
                Err(s) => return s,
            }
        } else {
            1
        };
        let match_mode = if arg_count > 3 {
            match self.get_number(&args[3], cell) {
                Ok(f) => {
                    if f == 0.0 {
                        Case::Sensitive
                    } else {
                        Case::Insensitive
                    }
                }
                Err(s) => return s,
            }
        } else {
            Case::Sensitive
        };

        let match_end = if arg_count > 4 {
            match self.get_number(&args[4], cell) {
                Ok(f) => f,
                Err(s) => return s,
            }
        } else {
            // disabled by default
            // the delimiter is specified in the formula
            0.0
        };
        if instance_num == 0 {
            return CalcResult::Error {
                error: Error::VALUE,
                origin: cell,
                message: "instance_num must be <> 0".to_string(),
            };
        }
        if delimiter.len() > text.len() {
            // so this is fun(!)
            // if the function was provided with two arguments is a #VALUE!
            // if it had more is a #N/A (irrespective of their values)
            if arg_count > 2 {
                return CalcResult::Error {
                    error: Error::VALUE,
                    origin: cell,
                    message: "The delimiter is longer than the text is trying to match".to_string(),
                };
            } else {
                return CalcResult::Error {
                    error: Error::NA,
                    origin: cell,
                    message: "The delimiter is longer than the text is trying to match".to_string(),
                };
            }
        }
        if match_end != 0.0 && match_end != 1.0 {
            return CalcResult::Error {
                error: Error::VALUE,
                origin: cell,
                message: "argument must be 0 or 1".to_string(),
            };
        };
        match text_after(&text, &delimiter, instance_num, match_mode) {
            Some(s) => CalcResult::String(s),
            None => {
                if match_end == 1.0 {
                    if instance_num == 1 {
                        return CalcResult::String("".to_string());
                    } else if instance_num == -1 {
                        return CalcResult::String(text);
                    }
                }
                if arg_count == 6 {
                    // An empty cell is converted to empty string (not 0)
                    match self.evaluate_node_in_context(&args[5], cell) {
                        CalcResult::EmptyCell => CalcResult::String("".to_string()),
                        result => result,
                    }
                } else {
                    CalcResult::Error {
                        error: Error::NA,
                        origin: cell,
                        message: "Value not found".to_string(),
                    }
                }
            }
        }
    }

    pub(crate) fn fn_textbefore(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(2..=6).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let text = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let delimiter = match self.get_string(&args[1], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let instance_num = if arg_count > 2 {
            match self.get_number(&args[2], cell) {
                Ok(f) => f.floor() as i32,
                Err(s) => return s,
            }
        } else {
            1
        };
        let match_mode = if arg_count > 3 {
            match self.get_number(&args[3], cell) {
                Ok(f) => {
                    if f == 0.0 {
                        Case::Sensitive
                    } else {
                        Case::Insensitive
                    }
                }
                Err(s) => return s,
            }
        } else {
            Case::Sensitive
        };

        let match_end = if arg_count > 4 {
            match self.get_number(&args[4], cell) {
                Ok(f) => f,
                Err(s) => return s,
            }
        } else {
            // disabled by default
            // the delimiter is specified in the formula
            0.0
        };
        if instance_num == 0 {
            return CalcResult::Error {
                error: Error::VALUE,
                origin: cell,
                message: "instance_num must be <> 0".to_string(),
            };
        }
        if delimiter.len() > text.len() {
            // so this is fun(!)
            // if the function was provided with two arguments is a #VALUE!
            // if it had more is a #N/A (irrespective of their values)
            if arg_count > 2 {
                return CalcResult::Error {
                    error: Error::VALUE,
                    origin: cell,
                    message: "The delimiter is longer than the text is trying to match".to_string(),
                };
            } else {
                return CalcResult::Error {
                    error: Error::NA,
                    origin: cell,
                    message: "The delimiter is longer than the text is trying to match".to_string(),
                };
            }
        }
        if match_end != 0.0 && match_end != 1.0 {
            return CalcResult::Error {
                error: Error::VALUE,
                origin: cell,
                message: "argument must be 0 or 1".to_string(),
            };
        };
        match text_before(&text, &delimiter, instance_num, match_mode) {
            Some(s) => CalcResult::String(s),
            None => {
                if match_end == 1.0 {
                    if instance_num == -1 {
                        return CalcResult::String("".to_string());
                    } else if instance_num == 1 {
                        return CalcResult::String(text);
                    }
                }
                if arg_count == 6 {
                    // An empty cell is converted to empty string (not 0)
                    match self.evaluate_node_in_context(&args[5], cell) {
                        CalcResult::EmptyCell => CalcResult::String("".to_string()),
                        result => result,
                    }
                } else {
                    CalcResult::Error {
                        error: Error::NA,
                        origin: cell,
                        message: "Value not found".to_string(),
                    }
                }
            }
        }
    }

    // TEXTJOIN(delimiter, ignore_empty, text1, [text2], …)
    pub(crate) fn fn_textjoin(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if arg_count < 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let delimiter = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let ignore_empty = match self.get_boolean(&args[1], cell) {
            Ok(b) => b,
            Err(error) => return error,
        };
        let mut values = Vec::new();
        for arg in &args[2..] {
            match self.evaluate_node_in_context(arg, cell) {
                CalcResult::Number(value) => values.push(format!("{value}")),
                CalcResult::Range { left, right } => {
                    if left.sheet != right.sheet {
                        return CalcResult::new_error(
                            Error::VALUE,
                            cell,
                            "Ranges are in different sheets".to_string(),
                        );
                    }
                    let row1 = left.row;
                    let mut row2 = right.row;
                    let column1 = left.column;
                    let mut column2 = right.column;
                    if row1 == 1 && row2 == LAST_ROW {
                        row2 = match self.workbook.worksheet(left.sheet) {
                            Ok(s) => s.dimension().max_row,
                            Err(_) => {
                                return CalcResult::new_error(
                                    Error::ERROR,
                                    cell,
                                    format!("Invalid worksheet index: '{}'", left.sheet),
                                );
                            }
                        };
                    }
                    if column1 == 1 && column2 == LAST_COLUMN {
                        column2 = match self.workbook.worksheet(left.sheet) {
                            Ok(s) => s.dimension().max_column,
                            Err(_) => {
                                return CalcResult::new_error(
                                    Error::ERROR,
                                    cell,
                                    format!("Invalid worksheet index: '{}'", left.sheet),
                                );
                            }
                        };
                    }
                    for row in row1..row2 + 1 {
                        for column in column1..(column2 + 1) {
                            match self.evaluate_cell(CellReferenceIndex {
                                sheet: left.sheet,
                                row,
                                column,
                            }) {
                                CalcResult::Number(value) => {
                                    values.push(format!("{value}"));
                                }
                                CalcResult::String(value) => values.push(value),
                                CalcResult::Boolean(value) => {
                                    if value {
                                        values.push("TRUE".to_string())
                                    } else {
                                        values.push("FALSE".to_string())
                                    }
                                }
                                CalcResult::EmptyCell => {
                                    if !ignore_empty {
                                        values.push("".to_string())
                                    }
                                }
                                error @ CalcResult::Error { .. } => return error,
                                CalcResult::EmptyArg | CalcResult::Range { .. } => {}
                                CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                                    return CalcResult::Error {
                                        error: Error::NIMPL,
                                        origin: cell,
                                        message: "Arrays not supported yet".to_string(),
                                    }
                                }
                            }
                        }
                    }
                }
                error @ CalcResult::Error { .. } => return error,
                CalcResult::String(value) => values.push(value),
                CalcResult::Boolean(value) => {
                    if value {
                        values.push("TRUE".to_string())
                    } else {
                        values.push("FALSE".to_string())
                    }
                }
                CalcResult::EmptyCell => {
                    if !ignore_empty {
                        values.push("".to_string())
                    }
                }
                CalcResult::EmptyArg => {}
                CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                    return CalcResult::Error {
                        error: Error::NIMPL,
                        origin: cell,
                        message: "Arrays not supported yet".to_string(),
                    }
                }
            };
        }
        let result = values.join(&delimiter);
        CalcResult::String(result)
    }

    fn read_delimiters(
        &mut self,
        node: &Node,
        cell: CellReferenceIndex,
    ) -> Result<Vec<String>, CalcResult> {
        let result = self.evaluate_node_in_context(node, cell);
        match result {
            CalcResult::Range { left, right } => {
                if left.sheet != right.sheet {
                    return Err(CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "Ranges are in different sheets".to_string(),
                    ));
                }
                let mut out = Vec::new();
                for row in left.row..=right.row {
                    for column in left.column..=right.column {
                        let value = self.evaluate_cell(CellReferenceIndex {
                            sheet: left.sheet,
                            row,
                            column,
                        });
                        match self.cast_to_string(value, cell) {
                            Ok(s) => out.push(s),
                            Err(e) => return Err(e),
                        }
                    }
                }
                Ok(out)
            }
            CalcResult::Array(array) => {
                let mut out = Vec::new();
                for row in array {
                    for node in row {
                        match node {
                            ArrayNode::String(s) => out.push(s),
                            ArrayNode::Number(f) => out.push(format!("{f}")),
                            ArrayNode::Boolean(b) => {
                                out.push(if b { "TRUE".to_string() } else { "FALSE".to_string() })
                            }
                            ArrayNode::Error(error) => {
                                return Err(CalcResult::new_error(error, cell, "".to_string()))
                            }
                        }
                    }
                }
                Ok(out)
            }
            error @ CalcResult::Error { .. } => Err(error),
            other => match self.cast_to_string(other, cell) {
                Ok(s) => Ok(vec![s]),
                Err(e) => Err(e),
            },
        }
    }

    // TEXTSPLIT(text, col_delimiter, [row_delimiter], [ignore_empty], [match_mode], [pad_with])
    pub(crate) fn fn_textsplit(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(2..=6).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let text = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let col_delimiters = match self.read_delimiters(&args[1], cell) {
            Ok(d) => d,
            Err(error) => return error,
        };
        let col_delimiters: Vec<String> =
            col_delimiters.into_iter().filter(|d| !d.is_empty()).collect();
        if col_delimiters.is_empty() {
            return CalcResult::new_error(Error::VALUE, cell, "Missing delimiter".to_string());
        }
        let row_delimiters: Vec<String> = if arg_count >= 3 {
            match self.read_delimiters(&args[2], cell) {
                Ok(d) => d.into_iter().filter(|d| !d.is_empty()).collect(),
                Err(error) => return error,
            }
        } else {
            Vec::new()
        };
        let ignore_empty = if arg_count >= 4 {
            match self.get_boolean(&args[3], cell) {
                Ok(b) => b,
                Err(error) => return error,
            }
        } else {
            false
        };
        let match_mode = if arg_count >= 5 {
            match self.get_number(&args[4], cell) {
                Ok(n) => n.trunc() as i64,
                Err(error) => return error,
            }
        } else {
            0
        };
        if match_mode != 0 && match_mode != 1 {
            return CalcResult::new_error(Error::VALUE, cell, "Invalid match_mode".to_string());
        }
        let pad_with = if arg_count >= 6 {
            let value = self.evaluate_node_in_context(&args[5], cell);
            match value {
                CalcResult::Number(f) => ArrayNode::Number(f),
                CalcResult::String(s) => ArrayNode::String(s),
                CalcResult::Boolean(b) => ArrayNode::Boolean(b),
                CalcResult::EmptyCell | CalcResult::EmptyArg => ArrayNode::String(String::new()),
                CalcResult::Error { error, .. } => ArrayNode::Error(error),
                _ => ArrayNode::Error(Error::VALUE),
            }
        } else {
            ArrayNode::Error(Error::NA)
        };
        let case_insensitive = match_mode == 1;
        let row_texts: Vec<String> = if row_delimiters.is_empty() {
            vec![text]
        } else {
            split_on_any(&text, &row_delimiters, case_insensitive)
        };
        let mut rows: Vec<Vec<ArrayNode>> = Vec::new();
        for row_text in row_texts {
            let mut fields = split_on_any(&row_text, &col_delimiters, case_insensitive);
            if ignore_empty {
                fields.retain(|f| !f.is_empty());
            }
            if ignore_empty && fields.is_empty() {
                continue;
            }
            rows.push(fields.into_iter().map(ArrayNode::String).collect());
        }
        if rows.is_empty() {
            rows.push(vec![ArrayNode::String(String::new())]);
        }
        let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
        for row in &mut rows {
            while row.len() < max_cols {
                row.push(pad_with.clone());
            }
        }
        CalcResult::Array(rows)
    }

    // SUBSTITUTE(text, old_text, new_text, [instance_num])
    pub(crate) fn fn_substitute(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if !(2..=4).contains(&arg_count) {
            return CalcResult::new_args_number_error(cell);
        }
        let text = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let old_text = match self.get_string(&args[1], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let new_text = match self.get_string(&args[2], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let instance_num = if arg_count > 3 {
            match self.get_number(&args[3], cell) {
                Ok(f) => Some(f.floor() as i32),
                Err(s) => return s,
            }
        } else {
            // means every instance is replaced
            None
        };
        if let Some(num) = instance_num {
            if num < 1 {
                return CalcResult::Error {
                    error: Error::VALUE,
                    origin: cell,
                    message: "Invalid value".to_string(),
                };
            }
            if old_text.is_empty() {
                return CalcResult::String(text);
            }
            CalcResult::String(substitute(&text, &old_text, &new_text, num))
        } else {
            if old_text.is_empty() {
                return CalcResult::String(text);
            }
            CalcResult::String(text.replace(&old_text, &new_text))
        }
    }
    pub(crate) fn fn_concatenate(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let arg_count = args.len();
        if arg_count == 0 {
            return CalcResult::new_args_number_error(cell);
        }
        let mut text_array = Vec::new();
        for arg in args {
            let text = match self.get_string(arg, cell) {
                Ok(s) => s,
                Err(error) => return error,
            };
            text_array.push(text)
        }
        CalcResult::String(text_array.join(""))
    }

    pub(crate) fn fn_exact(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let result1 = &self.evaluate_node_in_context(&args[0], cell);
        let result2 = &self.evaluate_node_in_context(&args[1], cell);
        // FIXME: Implicit intersection
        if let (CalcResult::Number(number1), CalcResult::Number(number2)) = (result1, result2) {
            // In Excel two numbers are the same if they are the same up to 15 digits.
            CalcResult::Boolean(to_precision(*number1, 15) == to_precision(*number2, 15))
        } else {
            let string1 = match self.cast_to_string(result1.clone(), cell) {
                Ok(s) => s,
                Err(error) => return error,
            };
            let string2 = match self.cast_to_string(result2.clone(), cell) {
                Ok(s) => s,
                Err(error) => return error,
            };
            CalcResult::Boolean(string1 == string2)
        }
    }
    // VALUE(text)
    pub(crate) fn fn_value(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        match self.evaluate_node_in_context(&args[0], cell) {
            CalcResult::String(text) => {
                let currencies = vec!["$", "€"];
                if let Ok((value, _)) = parse_formatted_number(&text, &currencies, self.locale) {
                    return CalcResult::Number(value);
                };
                CalcResult::Error {
                    error: Error::VALUE,
                    origin: cell,
                    message: "Invalid number".to_string(),
                }
            }
            CalcResult::Number(f) => CalcResult::Number(f),
            CalcResult::Boolean(_) => CalcResult::Error {
                error: Error::VALUE,
                origin: cell,
                message: "Invalid number".to_string(),
            },
            error @ CalcResult::Error { .. } => error,
            CalcResult::Range { .. } => {
                // TODO Implicit Intersection
                CalcResult::Error {
                    error: Error::VALUE,
                    origin: cell,
                    message: "Invalid number".to_string(),
                }
            }
            CalcResult::EmptyCell | CalcResult::EmptyArg => CalcResult::Number(0.0),
            CalcResult::Array(_) | CalcResult::Lambda { .. } => CalcResult::Error {
                error: Error::NIMPL,
                origin: cell,
                message: "Arrays not supported yet".to_string(),
            },
        }
    }

    pub(crate) fn fn_t(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        // FIXME: Implicit intersection
        let result = self.evaluate_node_in_context(&args[0], cell);
        match result {
            CalcResult::String(_) => result,
            error @ CalcResult::Error { .. } => error,
            _ => CalcResult::String("".to_string()),
        }
    }

    pub(crate) fn fn_valuetotext(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let strict = if args.len() == 2 {
            let format = match self.get_number(&args[1], cell) {
                Ok(f) => f.trunc(),
                Err(error) => return error,
            };
            if format == 0.0 {
                false
            } else if format == 1.0 {
                true
            } else {
                return CalcResult::new_error(Error::VALUE, cell, "Invalid format".to_string());
            }
        } else {
            false
        };
        let mut value = self.evaluate_node_in_context(&args[0], cell);
        if let CalcResult::Range { left, right } = value {
            if left.sheet != right.sheet {
                return CalcResult::new_error(
                    Error::VALUE,
                    cell,
                    "Ranges are in different sheets".to_string(),
                );
            }
            value = self.evaluate_cell(CellReferenceIndex {
                sheet: left.sheet,
                row: left.row,
                column: left.column,
            });
        }
        let text = match value {
            CalcResult::Number(f) => format!("{f}"),
            CalcResult::String(s) => {
                if strict {
                    format!("\"{s}\"")
                } else {
                    s
                }
            }
            CalcResult::Boolean(b) => {
                if b {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            CalcResult::EmptyCell | CalcResult::EmptyArg => {
                if strict {
                    "\"\"".to_string()
                } else {
                    "".to_string()
                }
            }
            CalcResult::Error { error, .. } => error.to_string(),
            CalcResult::Range { .. } | CalcResult::Array(_) | CalcResult::Lambda { .. } => {
                return CalcResult::Error {
                    error: Error::NIMPL,
                    origin: cell,
                    message: "Arrays not supported yet".to_string(),
                }
            }
        };
        CalcResult::String(text)
    }

    pub(crate) fn fn_arraytotext(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let strict = if args.len() == 2 {
            let format = match self.get_number(&args[1], cell) {
                Ok(f) => f.trunc(),
                Err(error) => return error,
            };
            if format == 0.0 {
                false
            } else if format == 1.0 {
                true
            } else {
                return CalcResult::new_error(Error::VALUE, cell, "Invalid format".to_string());
            }
        } else {
            false
        };
        let value = self.evaluate_node_in_context(&args[0], cell);
        let mut rows: Vec<Vec<String>> = Vec::new();
        match value {
            CalcResult::Range { left, right } => {
                if left.sheet != right.sheet {
                    return CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "Ranges are in different sheets".to_string(),
                    );
                }
                for row in left.row..=right.row {
                    let mut row_values = Vec::new();
                    for column in left.column..=right.column {
                        let v = self.evaluate_cell(CellReferenceIndex {
                            sheet: left.sheet,
                            row,
                            column,
                        });
                        row_values.push(array_to_text_value(&v, strict));
                    }
                    rows.push(row_values);
                }
            }
            CalcResult::Array(array) => {
                for row in array {
                    let mut row_values = Vec::new();
                    for v in row {
                        let text = match v {
                            ArrayNode::Number(f) => format!("{f}"),
                            ArrayNode::String(s) => {
                                if strict {
                                    format!("\"{s}\"")
                                } else {
                                    s
                                }
                            }
                            ArrayNode::Boolean(b) => {
                                if b {
                                    "TRUE".to_string()
                                } else {
                                    "FALSE".to_string()
                                }
                            }
                            ArrayNode::Error(error) => error.to_string(),
                        };
                        row_values.push(text);
                    }
                    rows.push(row_values);
                }
            }
            other => {
                rows.push(vec![array_to_text_value(&other, strict)]);
            }
        }
        let text = if strict {
            let rows_text: Vec<String> = rows
                .into_iter()
                .map(|row| row.join(","))
                .collect();
            format!("{{{}}}", rows_text.join(";"))
        } else {
            let flat: Vec<String> = rows.into_iter().flatten().collect();
            flat.join(", ")
        };
        CalcResult::String(text)
    }

    pub(crate) fn fn_bahttext(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        let number = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(error) => return error,
        };
        CalcResult::String(baht_text(number))
    }

    pub(crate) fn fn_char(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        let value = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(error) => return error,
        };
        let code = value.trunc();
        if !(1.0..=255.0).contains(&code) {
            return CalcResult::new_error(Error::VALUE, cell, "Number out of range".to_string());
        }
        match cp1252_to_char(code as u32) {
            Some(c) => CalcResult::String(c.to_string()),
            None => CalcResult::new_error(Error::VALUE, cell, "Invalid character".to_string()),
        }
    }

    pub(crate) fn fn_code(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        let text = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        match text.chars().next() {
            Some(c) => CalcResult::Number(char_to_cp1252(c) as f64),
            None => CalcResult::new_error(Error::VALUE, cell, "Empty string".to_string()),
        }
    }

    pub(crate) fn fn_clean(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        let text = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let result: String = text.chars().filter(|c| (*c as u32) >= 32).collect();
        CalcResult::String(result)
    }

    pub(crate) fn fn_asc(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        let text = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let result: String = text
            .chars()
            .map(|c| match c as u32 {
                0x3000 => ' ',
                code @ 0xFF01..=0xFF5E => char::from_u32(code - 0xFEE0).unwrap_or(c),
                _ => c,
            })
            .collect();
        CalcResult::String(result)
    }

    pub(crate) fn fn_jis(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        let text = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let result: String = text
            .chars()
            .map(|c| match c as u32 {
                0x0020 => '\u{3000}',
                code @ 0x0021..=0x007E => char::from_u32(code + 0xFEE0).unwrap_or(c),
                _ => c,
            })
            .collect();
        CalcResult::String(result)
    }

    pub(crate) fn fn_proper(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        let text = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let mut result = String::new();
        let mut prev_is_letter = false;
        for c in text.chars() {
            if c.is_alphabetic() {
                if prev_is_letter {
                    result.extend(c.to_lowercase());
                } else {
                    result.extend(c.to_uppercase());
                }
                prev_is_letter = true;
            } else {
                result.push(c);
                prev_is_letter = false;
            }
        }
        CalcResult::String(result)
    }

    pub(crate) fn fn_replace(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 4 {
            return CalcResult::new_args_number_error(cell);
        }
        let old_text = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        let start_num = match self.get_number(&args[1], cell) {
            Ok(f) => f.trunc(),
            Err(error) => return error,
        };
        let num_chars = match self.get_number(&args[2], cell) {
            Ok(f) => f.trunc(),
            Err(error) => return error,
        };
        let new_text = match self.get_string(&args[3], cell) {
            Ok(s) => s,
            Err(error) => return error,
        };
        if start_num < 1.0 || num_chars < 0.0 {
            return CalcResult::new_error(Error::VALUE, cell, "Invalid arguments".to_string());
        }
        let chars: Vec<char> = old_text.chars().collect();
        let start = (start_num as usize) - 1;
        let count = num_chars as usize;
        let mut result = String::new();
        for c in chars.iter().take(start.min(chars.len())) {
            result.push(*c);
        }
        result.push_str(&new_text);
        let tail_start = start.saturating_add(count);
        if tail_start < chars.len() {
            for c in &chars[tail_start..] {
                result.push(*c);
            }
        }
        CalcResult::String(result)
    }

    pub(crate) fn fn_fixed(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 3 {
            return CalcResult::new_args_number_error(cell);
        }
        let number = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(error) => return error,
        };
        let decimals = if args.len() >= 2 {
            match self.get_number(&args[1], cell) {
                Ok(f) => f.trunc() as i32,
                Err(error) => return error,
            }
        } else {
            2
        };
        let no_commas = if args.len() == 3 {
            match self.get_boolean(&args[2], cell) {
                Ok(b) => b,
                Err(error) => return error,
            }
        } else {
            false
        };
        let rounded = round_to_decimals(number, decimals);
        let group = &self.locale.numbers.symbols.group;
        let decimal = &self.locale.numbers.symbols.decimal;
        let minus = &self.locale.numbers.symbols.minus_sign;
        let magnitude =
            format_fixed_magnitude(rounded.abs(), decimals, !no_commas, group, decimal);
        let result = if rounded < 0.0 {
            format!("{minus}{magnitude}")
        } else {
            magnitude
        };
        CalcResult::String(result)
    }

    pub(crate) fn fn_dollar(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() || args.len() > 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let number = match self.get_number(&args[0], cell) {
            Ok(f) => f,
            Err(error) => return error,
        };
        let decimals = if args.len() == 2 {
            match self.get_number(&args[1], cell) {
                Ok(f) => f.trunc() as i32,
                Err(error) => return error,
            }
        } else {
            2
        };
        let rounded = round_to_decimals(number, decimals);
        let group = &self.locale.numbers.symbols.group;
        let decimal = &self.locale.numbers.symbols.decimal;
        let symbol = &self.locale.currency.symbol;
        let magnitude = format_fixed_magnitude(rounded.abs(), decimals, true, group, decimal);
        let result = if rounded < 0.0 {
            format!("({symbol}{magnitude})")
        } else {
            format!("{symbol}{magnitude}")
        };
        CalcResult::String(result)
    }
}
