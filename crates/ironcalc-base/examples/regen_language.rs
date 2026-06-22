use std::collections::BTreeMap;
use std::path::Path;

use ironcalc_base::language::Language;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/language");
    let json = std::fs::read_to_string(dir.join("language.json")).expect("read language.json");
    let languages: BTreeMap<String, Language> =
        serde_json::from_str(&json).expect("parse language.json");

    let encoded = bitcode::encode(&languages);
    let decoded: BTreeMap<String, Language> = bitcode::decode(&encoded).expect("decode round-trip");
    let en = decoded.get("en").expect("en locale");
    let names: std::collections::BTreeSet<_> = serde_json::to_value(&en.functions)
        .unwrap()
        .as_object()
        .unwrap()
        .values()
        .map(|v| v.as_str().unwrap().to_uppercase())
        .collect();

    let existing = std::fs::read(dir.join("language.bin")).expect("read language.bin");
    let changed = existing != encoded;

    std::fs::write(dir.join("language.bin"), &encoded).expect("write language.bin");
    println!(
        "regen-language: {} en functions, {} bytes, {}",
        names.len(),
        encoded.len(),
        if changed { "UPDATED" } else { "unchanged" }
    );
}
