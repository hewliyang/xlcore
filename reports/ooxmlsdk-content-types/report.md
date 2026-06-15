# ooxmlsdk emits an invalid `[Content_Types].xml` for created packages

## Summary

When a SpreadsheetML package is **created from scratch** with
[`KaiserY/ooxmlsdk`](https://github.com/KaiserY/ooxmlsdk) `0.7.0`, the resulting
`[Content_Types].xml`:

1. contains **no `<Default>` entries at all**, and
2. therefore declares **no content type for the relationship (`.rels`) parts**
   that the SDK itself writes (`_rels/.rels`, `xl/_rels/workbook.xml.rels`, …).

Per OPC (ISO/IEC 29500-2 §10.1.2.2.1) every part in a package must have a content
type, declared either by a `<Default>` (keyed by extension) or an `<Override>`
(keyed by part name). Because the `.rels` parts have neither, the package is
malformed. Microsoft Excel reports *"We found a problem with some content … Do
you want us to try to recover"* and silently repairs the file; the OpenXML SDK
validator reports `Pkg_RequiredPartDoNotExist`.

This is a regression in the `0.7.0` packaging rewrite (`SdkPackageStorage`); files
that are **loaded** and re-saved are unaffected, because `open()` parses the
existing `[Content_Types].xml` verbatim (Defaults included).

## Reproduction

Two minimal programs build the *same* logical workbook (workbook + 1 worksheet +
an empty stylesheet) and dump `[Content_Types].xml`.

- `rust-repro/`   — `ooxmlsdk` 0.7.0, `SpreadsheetDocument::create(...)` → `to_package_bytes()`
- `dotnet-repro/` — `DocumentFormat.OpenXml` 3.1.0, `SpreadsheetDocument.Create(...)` (the reference implementation)

```
cd rust-repro   && cargo run         # writes rust.xlsx
cd dotnet-repro && dotnet run        # writes dotnet.xlsx
```

### Output: ooxmlsdk (`out/rust-Content_Types.xml`) — INVALID

```xml
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Override ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml" PartName="/xl/workbook.xml"/>
  <Override ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"  PartName="/xl/worksheets/sheet1.xml"/>
  <Override ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"      PartName="/xl/styles1.xml"/>
</Types>
```

Package members written but **not covered** by any entry above:

```
_rels/.rels
xl/_rels/workbook.xml.rels
```

### Output: Open-XML-SDK (`out/dotnet-Content_Types.xml`) — VALID

```xml
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default  Extension="xml"  ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Default  Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/styles.xml"            ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
</Types>
```

### Validation (DocumentFormat.OpenXml `OpenXmlValidator`, Office2019)

| File                    | Result                                   |
| ----------------------- | ---------------------------------------- |
| `rust.xlsx`   (ooxmlsdk)| `Pkg_RequiredPartDoNotExist` — **1 error** |
| `dotnet.xlsx` (.NET SDK)| **0 errors**                             |

## What the diff tells us

The `.NET` output is the executable spec for the `Default`/`Override` decision:

| Part                  | Ext    | Decision in `dotnet.xlsx`                                  |
| --------------------- | ------ | --------------------------------------------------------- |
| `xl/workbook.xml`     | `xml`  | **first** `xml` part ⇒ becomes the `<Default Extension="xml">` (no Override needed) |
| `_rels/.rels` etc.    | `rels` | **first** `rels` part ⇒ becomes `<Default Extension="rels">` |
| `xl/worksheets/sheet1.xml` | `xml` | same ext, **different** content type ⇒ `<Override>`   |
| `xl/styles.xml`       | `xml`  | same ext, **different** content type ⇒ `<Override>`       |

Two behaviours ooxmlsdk is missing:

1. It never produces `<Default>` entries (everything is an `<Override>`).
2. It never registers a content type for the `.rels` parts it writes — so the
   required `<Default Extension="rels">` is absent. This is the part that makes
   the package invalid.

(A secondary, non-fatal divergence: ooxmlsdk names the styles part `styles1.xml`
while the reference SDK uses `styles.xml`.)

## Source of truth

`dotnet/Open-XML-SDK` does **not** implement this — it delegates to
`System.IO.Packaging`. The authoritative algorithm lives in **`dotnet/runtime`**:

> `src/libraries/System.IO.Packaging/src/System/IO/Packaging/ZipPackage.cs`
> → nested `class ContentTypeHelper`

Called by `CreatePartCore` (≈L65) for **every** part, including `.rels` parts:

```csharp
// ContentTypeHelper.AddContentType  (≈L811)
bool foundMatchingDefault = false;
string extension = partUri.PartUriExtension;

if (extension.Length == 0
    || (_defaultDictionary.TryGetValue(extension, out value)
        && !(foundMatchingDefault = value.AreTypeAndSubTypeEqual(contentType))))
{
    AddOverrideElement(partUri, contentType);     // <Override PartName=.../>
}
else if (!foundMatchingDefault)
{
    AddDefaultElement(extension, contentType);    // <Default Extension=.../>
}
// else: a matching Default already exists → emit nothing, reuse it
```

Rules:

1. First part of a given extension → create a `<Default Extension="…">`.
2. Same extension + same content type → emit nothing (reuse the Default).
3. Same extension + **different** content type, or an extensionless part → `<Override>`.
4. Serialization (≈L917-931): **all Defaults first, then all Overrides.**

Relationship parts (`.rels`) go through this same path, which is exactly why the
reference SDK always emits `<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml">`.

## The faulty part in ooxmlsdk

`ooxmlsdk-0.7.0/src/common/package.rs`:

| Concern              | Reference (.NET)                                   | ooxmlsdk 0.7.0 (faulty)                                            |
| -------------------- | ------------------------------------------------- | ----------------------------------------------------------------- |
| Content-type model   | `_defaultDictionary` (ext→CT) + `_overrideDictionary` (uri→CT) | `Types` holding **only** `Override`s — `empty_content_types()` (`L1937`), `content_types_from_raw_parts()` (`L1922`) |
| Per-part registration| `AddContentType` decides Default vs Override      | `push_part` → `add_content_type_override()` (`L1666`) — **always** Override, never Default |
| `.rels` parts        | registered via `AddContentType` ⇒ `<Default rels>`| `save_package` (`src/parts.rs` `L2258`) writes `_rels/.rels` and per-part `.rels` **directly, with no content-type registration** |

### Suggested fix

1. Replace `add_content_type_override` with an `add_content_type(path, content_type)`
   that ports `ContentTypeHelper.AddContentType` (extension dictionary first;
   `<Override>` only on content-type conflict or an extensionless part). This also
   makes media parts (`png`, `jpeg`, …) collapse into `<Default>` entries the way
   the reference SDK does.
2. Route every relationship part written by `save_package` through that same
   `add_content_type(rels_path, RELATIONSHIP_CONTENT_TYPE)` (the constant already
   exists at `package.rs:21`). This single change yields the required
   `<Default Extension="rels">` and makes the package valid.

## Downstream workaround (this repo)

Until upstream is fixed, `crates/xlcore-api/src/package_fix.rs` post-processes the
bytes from `to_package_bytes()` to inject the missing
`Default Extension="rels"` / `Default Extension="xml"` entries. With the shim,
`OpenXmlValidator` reports 0 errors and Excel opens the file without repair.

## Environment

- `ooxmlsdk` 0.7.0 (crates.io), `quick-xml` 0.40.1
- `DocumentFormat.OpenXml` 3.1.0, .NET 10
- Validator: `OpenXmlValidator(FileFormatVersions.Office2019)`
