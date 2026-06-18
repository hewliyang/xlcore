use crate::schema::ValidationDropdown;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
use std::collections::HashSet;
use xlcore_io::{parse_a1, parse_range};

pub fn extract(ws: &x::Worksheet) -> Vec<ValidationDropdown> {
    let mut out = Vec::new();
    let mut seen: HashSet<(u32, u32)> = HashSet::new();

    let Some(block) = ws.data_validations.as_ref() else {
        return out;
    };

    for dv in &block.data_validation {
        if !matches!(dv.r#type, Some(x::DataValidationValues::List)) {
            continue;
        }
        for sqref in &dv.sequence_of_references {
            let raw = sqref.as_str();
            let bounds = parse_range(raw).or_else(|| parse_a1(raw).map(|cell| (cell, cell)));
            let Some(((r1, c1), (r2, c2))) = bounds else {
                continue;
            };
            for r in r1..=r2 {
                for c in c1..=c2 {
                    if seen.insert((r, c)) {
                        out.push(ValidationDropdown { r, c });
                    }
                }
            }
        }
    }

    out
}
