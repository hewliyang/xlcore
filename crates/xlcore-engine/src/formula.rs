pub fn prepare_formula_for_ironcalc(formula: &str) -> String {
    let trimmed = formula.trim();
    let body = trimmed.strip_prefix('=').unwrap_or(trimmed);
    let rewritten = rewrite_compat_formula(body).unwrap_or_else(|| body.to_string());
    if trimmed.starts_with('=') {
        format!("={rewritten}")
    } else {
        rewritten
    }
}

fn rewrite_compat_formula(expr: &str) -> Option<String> {
    let expr = expr.trim();
    let (name, args_src) = parse_top_level_call(expr)?;
    if !is_let_name(name) {
        return None;
    }

    let args = split_function_args(args_src)?;
    if args.len() < 3 || args.len() % 2 == 0 {
        return None;
    }

    let mut bindings: Vec<(String, String)> = Vec::new();
    for pair in args[..args.len() - 1].chunks_exact(2) {
        let name = pair[0].trim();
        if !is_valid_let_binding_name(name) {
            return None;
        }
        let value_expr =
            rewrite_compat_formula(pair[1].trim()).unwrap_or_else(|| pair[1].trim().to_string());
        let value_expr = substitute_bindings(&value_expr, &bindings);
        bindings.push((name.to_ascii_uppercase(), value_expr));
    }

    let result = args.last()?.trim();
    let result = rewrite_compat_formula(result).unwrap_or_else(|| result.to_string());
    Some(substitute_bindings(&result, &bindings))
}

fn parse_top_level_call(expr: &str) -> Option<(&str, &str)> {
    let open = expr.find('(')?;
    if !expr.ends_with(')') {
        return None;
    }

    let name = expr[..open].trim();
    if name.is_empty() {
        return None;
    }

    let mut depth = 0i32;
    let mut in_string = false;
    let mut in_sheet_name = false;
    let mut matching_close = None;
    for (idx, ch) in expr.char_indices().skip(open) {
        if in_string {
            if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if in_sheet_name {
            if ch == '\'' {
                in_sheet_name = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '\'' => in_sheet_name = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    matching_close = Some(idx);
                    break;
                }
                if depth < 0 {
                    return None;
                }
            }
            _ => {}
        }
    }

    if matching_close? != expr.len() - 1 {
        return None;
    }

    Some((name, &expr[open + 1..expr.len() - 1]))
}

fn split_function_args(args: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut in_sheet_name = false;
    let mut chars = args.char_indices().peekable();

    while let Some((idx, ch)) = chars.next() {
        if in_string {
            if ch == '"' {
                if matches!(chars.peek(), Some((_, '"'))) {
                    chars.next();
                } else {
                    in_string = false;
                }
            }
            continue;
        }
        if in_sheet_name {
            if ch == '\'' {
                in_sheet_name = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '\'' => in_sheet_name = true,
            '(' | '{' | '[' => depth += 1,
            ')' | '}' | ']' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            ',' if depth == 0 => {
                out.push(args[start..idx].trim().to_string());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    if in_string || in_sheet_name || depth != 0 {
        return None;
    }

    out.push(args[start..].trim().to_string());
    Some(out)
}

fn substitute_bindings(expr: &str, bindings: &[(String, String)]) -> String {
    let mut out = String::with_capacity(expr.len());
    let chars: Vec<(usize, char)> = expr.char_indices().collect();
    let mut i = 0usize;
    let mut in_string = false;
    let mut in_sheet_name = false;

    while i < chars.len() {
        let (byte_idx, ch) = chars[i];
        if in_string {
            out.push(ch);
            if ch == '"' {
                if i + 1 < chars.len() && chars[i + 1].1 == '"' {
                    i += 1;
                    out.push(chars[i].1);
                } else {
                    in_string = false;
                }
            }
            i += 1;
            continue;
        }
        if in_sheet_name {
            out.push(ch);
            if ch == '\'' {
                in_sheet_name = false;
            }
            i += 1;
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                out.push(ch);
                i += 1;
            }
            '\'' => {
                in_sheet_name = true;
                out.push(ch);
                i += 1;
            }
            _ if is_identifier_start(ch) => {
                let mut end_i = i + 1;
                while end_i < chars.len() && is_identifier_continue(chars[end_i].1) {
                    end_i += 1;
                }
                let end_byte = chars
                    .get(end_i)
                    .map(|(idx, _)| *idx)
                    .unwrap_or_else(|| expr.len());
                let token = &expr[byte_idx..end_byte];
                let next = chars.get(end_i).map(|(_, c)| *c);
                let replacement = if next == Some('!') {
                    None
                } else {
                    bindings
                        .iter()
                        .rev()
                        .find(|(name, _)| token.eq_ignore_ascii_case(name))
                        .map(|(_, value)| value)
                };
                if let Some(value) = replacement {
                    out.push('(');
                    out.push_str(value);
                    out.push(')');
                } else {
                    out.push_str(token);
                }
                i = end_i;
            }
            _ => {
                out.push(ch);
                i += 1;
            }
        }
    }

    out
}

fn is_let_name(name: &str) -> bool {
    name.trim_start_matches("_xlfn.")
        .eq_ignore_ascii_case("LET")
}

fn is_valid_let_binding_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_identifier_start(first) {
        return false;
    }
    if !chars.all(is_identifier_continue) {
        return false;
    }
    !looks_like_a1_reference(name)
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '.'
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'
}

fn looks_like_a1_reference(name: &str) -> bool {
    let mut seen_digit = false;
    let mut seen_letter = false;
    for ch in name.chars() {
        if ch.is_ascii_alphabetic() && !seen_digit {
            seen_letter = true;
        } else if ch.is_ascii_digit() && seen_letter {
            seen_digit = true;
        } else {
            return false;
        }
    }
    seen_letter && seen_digit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_top_level_let() {
        assert_eq!(
            prepare_formula_for_ironcalc("=LET(total,SUM(A1:A2), total * 2)"),
            "=(SUM(A1:A2)) * 2"
        );
    }

    #[test]
    fn rewrites_prior_binding_references_inside_later_bindings() {
        assert_eq!(
            prepare_formula_for_ironcalc("LET(a,SUM(A1:A2), b, a*2, b+1)"),
            "((SUM(A1:A2))*2)+1"
        );
    }

    #[test]
    fn ignores_let_names_inside_strings_and_sheet_names() {
        assert_eq!(
            prepare_formula_for_ironcalc(r#"LET(x,1,CONCAT("x",'x'!A1,x))"#),
            r#"CONCAT("x",'x'!A1,(1))"#
        );
    }

    #[test]
    fn leaves_non_let_formulas_unchanged() {
        assert_eq!(prepare_formula_for_ironcalc("=SUM(A1:A2)"), "=SUM(A1:A2)");
    }
}
