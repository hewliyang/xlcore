use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiError, ApiErrorCode, SheetProtectionInfo, SheetProtectionPatch, WorkbookProtectionInfo,
    WorkbookProtectionPatch,
};

use crate::errors::sdk_err_to_api;
use crate::{Result, Workbook};

impl Workbook {
    pub fn sheet_protection(
        &mut self,
        sheet: impl AsRef<str>,
    ) -> Result<Option<SheetProtectionInfo>> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        Ok(ws
            .x_sheet_protection
            .as_ref()
            .map(|sp| read_sheet_protection(&sheet, sp)))
    }

    pub fn set_sheet_protection(
        &mut self,
        sheet: impl AsRef<str>,
        patch: SheetProtectionPatch,
    ) -> Result<SheetProtectionInfo> {
        validate_sheet_patch(&patch)?;
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let sp = ws
            .x_sheet_protection
            .get_or_insert_with(x::SheetProtection::default);
        apply_sheet_patch(sp, &patch);
        Ok(read_sheet_protection(&sheet, sp))
    }

    pub fn remove_sheet_protection(
        &mut self,
        sheet: impl AsRef<str>,
    ) -> Result<Option<SheetProtectionInfo>> {
        let sheet = sheet.as_ref().to_string();
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let removed = ws
            .x_sheet_protection
            .as_ref()
            .map(|sp| read_sheet_protection(&sheet, sp));
        ws.x_sheet_protection = None;
        Ok(removed)
    }

    pub fn workbook_protection(&mut self) -> Result<Option<WorkbookProtectionInfo>> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let wb = wb_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        Ok(wb.workbook_protection.as_ref().map(read_workbook_protection))
    }

    pub fn set_workbook_protection(
        &mut self,
        patch: WorkbookProtectionPatch,
    ) -> Result<WorkbookProtectionInfo> {
        validate_workbook_patch(&patch)?;
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let wb = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let wp = wb
            .workbook_protection
            .get_or_insert_with(x::WorkbookProtection::default);
        apply_workbook_patch(wp, &patch);
        Ok(read_workbook_protection(wp))
    }

    pub fn remove_workbook_protection(&mut self) -> Result<Option<WorkbookProtectionInfo>> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let wb = wb_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let removed = wb.workbook_protection.as_ref().map(read_workbook_protection);
        wb.workbook_protection = None;
        Ok(removed)
    }
}

fn read_sheet_protection(sheet: &str, sp: &x::SheetProtection) -> SheetProtectionInfo {
    SheetProtectionInfo {
        sheet: sheet.to_string(),
        enabled: sp.sheet.unwrap_or(false),
        password: opt_clone(sp.password.as_ref()),
        algorithm_name: opt_clone(sp.algorithm_name.as_ref()),
        hash_value: opt_clone(sp.hash_value.as_ref()),
        salt_value: opt_clone(sp.salt_value.as_ref()),
        spin_count: sp.spin_count,
        objects: sp.objects,
        scenarios: sp.scenarios,
        format_cells: sp.format_cells,
        format_columns: sp.format_columns,
        format_rows: sp.format_rows,
        insert_columns: sp.insert_columns,
        insert_rows: sp.insert_rows,
        insert_hyperlinks: sp.insert_hyperlinks,
        delete_columns: sp.delete_columns,
        delete_rows: sp.delete_rows,
        select_locked_cells: sp.select_locked_cells,
        sort: sp.sort,
        auto_filter: sp.auto_filter,
        pivot_tables: sp.pivot_tables,
        select_unlocked_cells: sp.select_unlocked_cells,
    }
}

fn apply_sheet_patch(sp: &mut x::SheetProtection, patch: &SheetProtectionPatch) {
    if let Some(v) = patch.enabled {
        sp.sheet = Some(v);
    }
    if let Some(v) = patch.password.clone() {
        sp.password = Some(v);
    }
    if let Some(v) = patch.algorithm_name.clone() {
        sp.algorithm_name = Some(v);
    }
    if let Some(v) = patch.hash_value.clone() {
        sp.hash_value = Some(v);
    }
    if let Some(v) = patch.salt_value.clone() {
        sp.salt_value = Some(v);
    }
    if let Some(v) = patch.spin_count {
        sp.spin_count = Some(v);
    }
    if let Some(v) = patch.objects {
        sp.objects = Some(v);
    }
    if let Some(v) = patch.scenarios {
        sp.scenarios = Some(v);
    }
    if let Some(v) = patch.format_cells {
        sp.format_cells = Some(v);
    }
    if let Some(v) = patch.format_columns {
        sp.format_columns = Some(v);
    }
    if let Some(v) = patch.format_rows {
        sp.format_rows = Some(v);
    }
    if let Some(v) = patch.insert_columns {
        sp.insert_columns = Some(v);
    }
    if let Some(v) = patch.insert_rows {
        sp.insert_rows = Some(v);
    }
    if let Some(v) = patch.insert_hyperlinks {
        sp.insert_hyperlinks = Some(v);
    }
    if let Some(v) = patch.delete_columns {
        sp.delete_columns = Some(v);
    }
    if let Some(v) = patch.delete_rows {
        sp.delete_rows = Some(v);
    }
    if let Some(v) = patch.select_locked_cells {
        sp.select_locked_cells = Some(v);
    }
    if let Some(v) = patch.sort {
        sp.sort = Some(v);
    }
    if let Some(v) = patch.auto_filter {
        sp.auto_filter = Some(v);
    }
    if let Some(v) = patch.pivot_tables {
        sp.pivot_tables = Some(v);
    }
    if let Some(v) = patch.select_unlocked_cells {
        sp.select_unlocked_cells = Some(v);
    }
}

fn read_workbook_protection(wp: &x::WorkbookProtection) -> WorkbookProtectionInfo {
    WorkbookProtectionInfo {
        lock_structure: wp.lock_structure,
        lock_windows: wp.lock_windows,
        lock_revision: wp.lock_revision,
        workbook_password: opt_clone(wp.workbook_password.as_ref()),
        workbook_algorithm_name: opt_clone(wp.workbook_algorithm_name.as_ref()),
        workbook_hash_value: opt_clone(wp.workbook_hash_value.as_ref()),
        workbook_salt_value: opt_clone(wp.workbook_salt_value.as_ref()),
        workbook_spin_count: wp.workbook_spin_count,
        revisions_password: opt_clone(wp.revisions_password.as_ref()),
        revisions_algorithm_name: opt_clone(wp.revisions_algorithm_name.as_ref()),
        revisions_hash_value: opt_clone(wp.revisions_hash_value.as_ref()),
        revisions_salt_value: opt_clone(wp.revisions_salt_value.as_ref()),
        revisions_spin_count: wp.revisions_spin_count,
    }
}

fn apply_workbook_patch(wp: &mut x::WorkbookProtection, patch: &WorkbookProtectionPatch) {
    if let Some(v) = patch.lock_structure {
        wp.lock_structure = Some(v);
    }
    if let Some(v) = patch.lock_windows {
        wp.lock_windows = Some(v);
    }
    if let Some(v) = patch.lock_revision {
        wp.lock_revision = Some(v);
    }
    if let Some(v) = patch.workbook_password.clone() {
        wp.workbook_password = Some(v);
    }
    if let Some(v) = patch.workbook_algorithm_name.clone() {
        wp.workbook_algorithm_name = Some(v);
    }
    if let Some(v) = patch.workbook_hash_value.clone() {
        wp.workbook_hash_value = Some(v);
    }
    if let Some(v) = patch.workbook_salt_value.clone() {
        wp.workbook_salt_value = Some(v);
    }
    if let Some(v) = patch.workbook_spin_count {
        wp.workbook_spin_count = Some(v);
    }
    if let Some(v) = patch.revisions_password.clone() {
        wp.revisions_password = Some(v);
    }
    if let Some(v) = patch.revisions_algorithm_name.clone() {
        wp.revisions_algorithm_name = Some(v);
    }
    if let Some(v) = patch.revisions_hash_value.clone() {
        wp.revisions_hash_value = Some(v);
    }
    if let Some(v) = patch.revisions_salt_value.clone() {
        wp.revisions_salt_value = Some(v);
    }
    if let Some(v) = patch.revisions_spin_count {
        wp.revisions_spin_count = Some(v);
    }
}

fn validate_sheet_patch(patch: &SheetProtectionPatch) -> Result<()> {
    if let Some(p) = patch.password.as_deref() {
        validate_hex(p, "password")?;
    }
    if let Some(p) = patch.algorithm_name.as_deref() {
        validate_non_empty(p, "algorithmName")?;
    }
    if let Some(p) = patch.hash_value.as_deref() {
        validate_non_empty(p, "hashValue")?;
    }
    if let Some(p) = patch.salt_value.as_deref() {
        validate_non_empty(p, "saltValue")?;
    }
    Ok(())
}

fn validate_workbook_patch(patch: &WorkbookProtectionPatch) -> Result<()> {
    if let Some(p) = patch.workbook_password.as_deref() {
        validate_hex(p, "workbookPassword")?;
    }
    if let Some(p) = patch.revisions_password.as_deref() {
        validate_hex(p, "revisionsPassword")?;
    }
    for (value, name) in [
        (patch.workbook_algorithm_name.as_deref(), "workbookAlgorithmName"),
        (patch.workbook_hash_value.as_deref(), "workbookHashValue"),
        (patch.workbook_salt_value.as_deref(), "workbookSaltValue"),
        (patch.revisions_algorithm_name.as_deref(), "revisionsAlgorithmName"),
        (patch.revisions_hash_value.as_deref(), "revisionsHashValue"),
        (patch.revisions_salt_value.as_deref(), "revisionsSaltValue"),
    ] {
        if let Some(v) = value {
            validate_non_empty(v, name)?;
        }
    }
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidProtection,
            format!("{field} must not be empty"),
        ));
    }
    Ok(())
}

fn validate_hex(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidProtection,
            format!("{field} must not be empty"),
        ));
    }
    if !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::new(
            ApiErrorCode::InvalidProtection,
            format!("{field} must be a hexadecimal string"),
        ));
    }
    Ok(())
}

fn opt_clone(value: Option<&String>) -> Option<String> {
    value.filter(|s| !s.is_empty()).cloned()
}
