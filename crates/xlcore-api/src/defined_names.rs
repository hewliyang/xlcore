use ooxmlsdk::simple_type::BooleanValue;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{ApiError, ApiErrorCode, ApiWarning, DefinedNameInfo, DefinedNamePatch};

use crate::errors::sdk_err_to_api;
use crate::{Result, Workbook};

impl Workbook {
    pub fn defined_names(&mut self) -> Result<Vec<DefinedNameInfo>> {
        let sheet_names = self.sheet_names()?;
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let wb = wb_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(dns) = wb.defined_names.as_ref() else {
            return Ok(Vec::new());
        };
        Ok(dns
            .defined_name
            .iter()
            .map(|dn| defined_name_info(dn, &sheet_names))
            .collect())
    }

    pub fn set_defined_name(&mut self, patch: DefinedNamePatch) -> Result<DefinedNameInfo> {
        validate_defined_name(&patch)?;
        let sheet_names = self.sheet_names()?;
        let local_sheet_id = match patch.scope.as_deref() {
            None => None,
            Some(sheet) => Some(resolve_scope(sheet, &sheet_names)?),
        };

        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let wb = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let dns = wb
            .defined_names
            .get_or_insert_with(x::DefinedNames::default);
        let trimmed_formula = patch.reference.trim().to_string();
        let engine_supported = looks_like_reference_formula(&trimmed_formula);
        let pos = dns
            .defined_name
            .iter()
            .position(|dn| dn.name.as_str() == patch.name && dn.local_sheet_id == local_sheet_id);
        let comment = patch.comment.clone().map(Into::into);
        let hidden = patch.hidden.map(|h| h.into());
        if let Some(idx) = pos {
            let existing = &mut dns.defined_name[idx];
            existing.xml_content = Some(trimmed_formula.clone().into());
            existing.comment = comment;
            existing.hidden = hidden;
        } else {
            dns.defined_name.push(x::DefinedName {
                name: patch.name.clone(),
                local_sheet_id,
                xml_content: Some(trimmed_formula.clone().into()),
                comment,
                hidden,
                ..Default::default()
            });
        }
        let info = DefinedNameInfo {
            name: patch.name.clone(),
            reference: trimmed_formula.clone(),
            scope: patch.scope.clone(),
            comment: patch.comment,
            hidden: patch.hidden.unwrap_or(false),
        };
        if !engine_supported {
            let mut warning = ApiWarning::new(
                ApiErrorCode::LossyOperation,
                format!(
                    "defined name '{}' uses a non-reference expression ('{}'); calc engine will resolve it as #NAME?",
                    patch.name, trimmed_formula
                ),
            );
            if let Some(scope) = patch.scope.as_deref() {
                warning = warning.with_sheet(scope);
            }
            self.push_warning(warning);
        }
        Ok(info)
    }

    pub fn remove_defined_name(
        &mut self,
        name: impl AsRef<str>,
        scope: Option<&str>,
    ) -> Result<Option<DefinedNameInfo>> {
        let name = name.as_ref();
        let sheet_names = self.sheet_names()?;
        let local_sheet_id = match scope {
            None => None,
            Some(sheet) => Some(resolve_scope(sheet, &sheet_names)?),
        };

        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let wb = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(dns) = wb.defined_names.as_mut() else {
            return Ok(None);
        };
        let pos = dns
            .defined_name
            .iter()
            .position(|dn| dn.name.as_str() == name && dn.local_sheet_id == local_sheet_id);
        let removed = pos.map(|idx| {
            let dn = dns.defined_name.remove(idx);
            defined_name_info(&dn, &sheet_names)
        });
        if dns.defined_name.is_empty() {
            wb.defined_names = None;
        }
        Ok(removed)
    }

    fn sheet_names(&mut self) -> Result<Vec<String>> {
        Ok(self
            .workbook_sheets()?
            .iter()
            .map(|s| s.name.as_str().to_string())
            .collect())
    }
}

fn defined_name_info(dn: &x::DefinedName, sheet_names: &[String]) -> DefinedNameInfo {
    let scope = dn
        .local_sheet_id
        .and_then(|i| sheet_names.get(i as usize).cloned());
    DefinedNameInfo {
        name: dn.name.as_str().to_string(),
        reference: dn
            .xml_content
            .as_ref()
            .map(|s| s.as_str().to_string())
            .unwrap_or_default(),
        scope,
        comment: dn.comment.as_ref().map(|s| s.as_str().to_string()),
        hidden: bool::from(dn.hidden.unwrap_or(BooleanValue::from_bool(false))),
    }
}

fn resolve_scope(sheet: &str, sheet_names: &[String]) -> Result<u32> {
    sheet_names
        .iter()
        .position(|s| s == sheet)
        .map(|i| i as u32)
        .ok_or_else(|| {
            ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found for scope: {sheet}"),
            )
            .with_sheet(sheet)
        })
}

fn validate_defined_name(patch: &DefinedNamePatch) -> Result<()> {
    let name = patch.name.as_str();
    if name.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidDefinedName,
            "defined name is empty",
        ));
    }
    if name.len() > 255 {
        return Err(ApiError::new(
            ApiErrorCode::InvalidDefinedName,
            "defined name exceeds 255 characters",
        ));
    }
    let first = name.chars().next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '_' || first == '\\') {
        return Err(ApiError::new(
            ApiErrorCode::InvalidDefinedName,
            "defined name must start with a letter, underscore, or backslash",
        ));
    }
    if name.chars().any(|c| c.is_whitespace()) {
        return Err(ApiError::new(
            ApiErrorCode::InvalidDefinedName,
            "defined name cannot contain whitespace",
        ));
    }
    for c in name.chars() {
        let ok = c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '\\' || c == '?';
        if !ok {
            return Err(ApiError::new(
                ApiErrorCode::InvalidDefinedName,
                format!("defined name contains invalid character: {c:?}"),
            ));
        }
    }
    if looks_like_cell_ref(name) {
        return Err(ApiError::new(
            ApiErrorCode::InvalidDefinedName,
            "defined name cannot look like a cell reference",
        ));
    }
    let upper = name.to_ascii_uppercase();
    if matches!(upper.as_str(), "R" | "C") {
        return Err(ApiError::new(
            ApiErrorCode::InvalidDefinedName,
            "defined name cannot be a reserved single letter (R/C)",
        ));
    }
    if patch.reference.trim().is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidDefinedName,
            "defined name reference is empty",
        ));
    }
    Ok(())
}

fn looks_like_reference_formula(formula: &str) -> bool {
    let body = formula.strip_prefix('=').unwrap_or(formula).trim();
    if body.is_empty() {
        return false;
    }
    let (_sheet, rest) = split_sheet_prefix(body);
    let mut parts = rest.splitn(2, ':');
    let Some(first) = parts.next() else {
        return false;
    };
    if !is_cell_token(first) {
        return false;
    }
    if let Some(second) = parts.next() {
        if !is_cell_token(second) {
            return false;
        }
    }
    true
}

fn split_sheet_prefix(s: &str) -> (Option<&str>, &str) {
    if let Some(rest) = s.strip_prefix('\'') {
        if let Some(end) = rest.find('\'') {
            let after = &rest[end + 1..];
            if let Some(rest2) = after.strip_prefix('!') {
                return (Some(&rest[..end]), rest2);
            }
        }
        return (None, s);
    }
    if let Some(idx) = s.find('!') {
        let prefix = &s[..idx];
        if prefix
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
            && !prefix.is_empty()
        {
            return (Some(prefix), &s[idx + 1..]);
        }
    }
    (None, s)
}

fn is_cell_token(token: &str) -> bool {
    let token = token.trim();
    let token = token.strip_prefix('$').unwrap_or(token);
    let mut chars = token.chars().peekable();
    let mut letters = 0;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_alphabetic() {
            letters += 1;
            chars.next();
        } else {
            break;
        }
    }
    if letters == 0 || letters > 3 {
        return false;
    }
    if let Some(&'$') = chars.peek() {
        chars.next();
    }
    let mut digits = 0;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            digits += 1;
            chars.next();
        } else {
            break;
        }
    }
    digits > 0 && chars.next().is_none()
}

fn looks_like_cell_ref(name: &str) -> bool {
    let mut chars = name.chars().peekable();
    let mut letters = 0;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_alphabetic() {
            letters += 1;
            chars.next();
        } else {
            break;
        }
    }
    if letters == 0 || letters > 3 {
        return false;
    }
    let mut digits = 0;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            digits += 1;
            chars.next();
        } else {
            break;
        }
    }
    digits > 0 && chars.next().is_none()
}
