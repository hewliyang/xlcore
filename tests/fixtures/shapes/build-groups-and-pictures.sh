#!/usr/bin/env bash
# Fixture: groups (incl. nested groups + nested pictures) and standalone
# pictures, with overlapping shapes laid out to lock in z-order.
#
# The shape extractor (`crates/xlcore-export/src/shapes.rs`) and renderer
# (`packages/xlsx-preview/src/shape.ts`) already claim:
#
#   - top-level `xdr:grpSp` flattening with `xfrm/off/ext/chOff/chExt`,
#   - nested `xdr:sp` / `xdr:grpSp` / `xdr:cxnSp` inside a group,
#   - nested `xdr:pic` (incl. `<a:srcRect>` crop) inside a group,
#   - z-order = XML traversal order.
#
# Each of those was previously exercised only indirectly through other
# fixtures. This file is the dedicated regression baseline.
#
# Rows / panels (left → right, top → bottom):
#
#   A. Z-order: 3 shapes (rect/oval/diamond) at the same anchor, each
#      overlapping the previous by ~50%. The diamond is added last and
#      must paint on top of the oval; the oval over the rect. Painted
#      with distinct accent fills so any swap is loud.
#
#   B. Standalone pictures: two raster PNGs added via
#      `sheet.pictures.add(...)` (the `xdr:pic` "loose" path — no `xfrm`,
#      anchor-derived geometry) and one via
#      `sheet.shapes.addPictureShape(...)` (the `xdr:pic` "with xfrm"
#      path).
#
#   C. Group of 3 shapes: an accent-filled rect, an oval, and a chevron
#      grouped together (`xdr:grpSp` with three nested `xdr:sp`).
#      Exercises `chOff` / `chExt` mapping of child logical coords back
#      to the group bbox.
#
#   D. Group containing a picture: a label rect + a picture nested under
#      one `xdr:grpSp` — the "nested `xdr:pic` in groups" code path
#      shapes.rs already special-cases.
#
#   E. Nested groups: an outer group whose children are (a) one rect and
#      (b) an inner group containing two more shapes. Exercises the
#      recursive walk in `visit_shapes` and the compounded `chOff/chExt`
#      math.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/groups-and-pictures.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

# Two tiny distinct PNGs (128×128 each, white diagonal hatching over a
# solid color with a black border + a single-letter label). Generated
# once and inlined as data URLs so the fixture has no external deps and
# the script is hermetic.
read -r -d '' PNG_BLUE <<'B64' || true
data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAIAAAACACAIAAABMXPacAAADbklEQVR4nO2dPYsTURSG78Sx2EYECxcLN3/AIqVYpBVGECxFC/uU4i6CfYS10T8QEFysd/MzbCytNlpowEq2EEVHZjOMMfOZeDNnzr3PWy3Lm93Lec/9GB7mJtjbPzkdR/2DqWks/BbrE8RxvPip4WdOxxF+i/XpZb7sk9XCb7c+PWoq23PB3v5J5g6CoMk/QP+prObpDFhorX0Y2dLfAJCIwsLfLo5GnIvsngMLV/iqGcC5yO6eXKjiADhrtpZB6Qwgg3YyqFqCyKBa69Yn07K/5hREBtvOoP4YSgZbzaDRcwAZbK8+Pc6asj2XzAAyEMwgXYLIQCoDeECXeEBDsSdbrE+6BJGBVM9tzgOYB1bqU/wcwJ7cWs+VPoiRQTsZwANMa2vpGgGwvlcLHlCvzvbQiuAB7WWQCR7QoQzgAcIZwAOE9w94QLHgAYrPOfAATRnAA2oED3DcDw+IZDOABxjZeQAPEM4AHjCV9cMDDDxA2bkFHhB1qqbwADUZZIIHlAoeoK+v4QH6MoAH1Ase4Iuf9wOqBA/Q2te8H6AmA3hAJJsBPMDAA3SsFfCAyEk/PMDAAzzya70v6OOLezvXbwyHw8Fg8OrWb/HxeMcDggsXr94fz24+mUwmo9FIfDyb+V3gAXePPoVh2J3xePd+wPfZ+7PBg+6MxxceEP/6OX9z8OX14/nbZ9/eHYuPx7v3A4LzPWD34eG1Ry9/fP4gPh5/7wvq7VwKL+92Zzy+8ID4fAmaHz39enx45fZIfDwW/cnt6fm7dZev984r81c8X+A3ufrMnt/Jl5f7guoFD3DZDw8w8ACv/bwfMJXNAB5g4AEe+bXyAOOQXyUPcMPvAg/oK/er5wFGv18lD3DYr4MHOOxXxgPc8+vgAQ77uS9IOAN4QL3gAS774QEGHuC1Hx4wlc0AHmDgAR7584IHpIIHKO5reIDiDOABq4IH6OtreIDiDOABpYIHKO5reICaDOAB9YIH6OtreICaDPj+gETwAH/98AADD/DInxc8IBU8QHFfwwMUZwAPWBU8QF9fwwMUZwAPKBU8QHFfwwPUZAAPqBc8QF9fwwMUZQAPSAQP8NcPDzCyPCC5tjJ/ryLaqlavrUSCIgABLa9d/yxB1e7qtQx/f6P6wAOEz0X1M2DlrzMP7NaH+4JMoVo7m/4BlVsp8zLmHM8AAAAASUVORK5CYII=
B64

read -r -d '' PNG_ORANGE <<'B64' || true
data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAIAAAACACAIAAABMXPacAAADqElEQVR4nO2cv48SQRiGZ4GQi4VcozE25mKn1R0VamMuOeM/YWH8FzxLsYRrjC25+KO3NTEnpbEjVlhZ01i4Gi94StbA3QLnsssAy85+M8/bcXnnsvneb2YHnsx4wdOLqu6rekXpC3969fGCIAiH6Y2p+/hTrE9h4huPTBb+VOtTWGIM/jlapJ7e8B0Quj3PmzsAra5Jzc9mwKkWeg+jlDQVADKh0sy/nm2N2Belug+cucInzgDeyTO19J5llmICYF+UVQbxM4AMMskgeQni+4FKsz6Tgb72LogM1pyBxjaUDNaZgd73ADJYW30K7DXN9tzpj3H8Bmcsg3AJIgNDGcADcsUDNMU7Ob36jJcgMjDTcyvwAOZBGvWJ+zGOfVFGPZfwYxwZZJEBPEBlt5YuEADre7LgAfOV2x46L3hAhhlMBsIDcpMBPMBwBvAAw+8PeECM4AGC9znwAEEZwAPmCR5gtx8e4JvNAB6gzM4DeIDhDOABFbN+eICCB7jkj4jzAaHgAYL7Gh4gNwN4QETwAHl9DQ+QmwHnA+IFD5Db1/AAMRlwPkBD8ABxfQ0PEJMBPGAkeICzfniAgge45BfKA15+/lO9WqrVatVq9fXWC+PPs5Jf3PmA91//HnZO2g8ufLrXbbfbrVbrw523Bp9neb/Q8wEHH08O9jY2N4Y33m0+v9ZsNhuNRl5q6gIP+PJtsH2lOP648+5+t9s1+Dyr+qWfDwiU8n728vM89p8PuHGp2OkNxh87vcHNy8U819Q2HvD4Vnn/qO//Ht4l+70fPDnq798uG3we5+4L2rteerhdvvvquHb4a/fN8aOd8u5WKc81Xcg/uj09crfu9PXes4b5c79f4I/Wx3v2I+b2dM065riPlFg/5wN8sxlwPkDBA5z2cz6gYjYDeICCB7jkF8oDrPKL4wH2+IXyAKv8EnmAbX7pPMAyvwAeYLdfAA+w2y+AB9jt574gwxnAAzQED7DYDw9Q8ACn/fCAitkM4AEKHuCSPyJ4QCh4gOC+hgfIzQAeEBE8QF5fwwPkZgAPiBc8QG5fwwPEZAAP0BA8QFxfwwPEZMB9QSPBA5z1wwMUPMAlf0TwgFDwAMF9DQ+QmwE8ICJ4gLy+hgfIzQAeEC94gNy+hgeIyQAeoCF4gLi+hgeIyQAeMBI8wFk/PECZ5QGjaysj9yqitSpybSUyJwIwoam169wSNMeduJbhV0vVBx5geF+kMQP+++/Mg1Trw31BKqZAGc2Df8aUmcpyk4TmAAAAAElFTkSuQmCC
B64

read -r -d '' PNG_GREEN <<'B64' || true
data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAIAAAACACAIAAABMXPacAAADlklEQVR4nO2dsW7aUBSGrwFFkAlFkdKlicSMH6AsVOpSsbUvkEoM2TpnyAO0b5HmAbrSpVJfoJPTKYkYqg6RKEOSIYmEqCwsl2JjQ2p877n3+ycU/RB0/nOvr/XJB6992g4OA/+Tr1YW/gLr402n09mrFd8THAb4C6xPJfbF78wW/mLrU6GmenvOa5+2Y7fneav8A/SfimserYCZ1roOo6L0NwCkRbXUv86ORpyLij0Hpu7wWSuAc1Gx1+RUpQfAWbO0DJauADIoJ4OsLYgMsrVufWLN+3NOQWSw6Qzyj6FksNEMVroPIIPN1afCWVNvz4UrgAw0ZhBtQWSgKwN4gEk8YEVxTS6wPtEWRAa6eu7pPIB1UEh90u8DuCaX1nNLb8TIoJwM4AGqtL10jQDY37MFD8iXsT20IHhAeRnEggcYlAE8QHMG8ADN1w94QLrgAYLPOfAASRnAA3IED7DcDw8I9GYAD1B61wE8QHMG8ABfrx8eoOABws4t8IDAqJrCA8RkEAsesFTwAHl9DQ+QlwE8IF/wAFf8PB+QJXiA1L7m+QAxGcADAr0ZwAMUPEDGXgEPCKz0wwMUPMAhv9R5QeNv4+3WdqfT6fV6g1cD7d/HLR5w9+Pu5vtN66R1e3Tb7Xb7/b5pNbWcB4wGo723e141nHh31jxrNBqTycSQmjrxfMDDr4f683r0deuVqzdX1WrVnJrazwOmk2i8+OjLaPhheHF8YVpNLecBW8+27n/eK6V2X+/uv99//P1oeE1tmxe083Ln+vP1bB2Mv469imd4TW3jAc0XzcZB4/LkcvhxWGvW4gCMrela/nB6enK27vx476Rif8b9BX6VqM/5u/NkeZkXlC94gM1+eICCBzjt5/kAX28G8AAFD3DIL5UHKIv88niANX6pPMAmv0geYJlfHg+w2y+AB9jtF8AD7PYL4AF2+5kXpDkDeEC+4AE2++EBCh7gtB8e4OvNAB6g4AEO+ZOCB0SCBwjua3iA4AzgAYuCB8jra3iA4AzgAUsFDxDc1/AAMRnAA/IFD5DX1/AAMRnw+wGh4AHu+uEBCh7gkD8peEAkeIDgvoYHCM4AHrAoeIC8voYHCM4AHrBU8ADBfQ0PEJMBPCBf8AB5fQ0PEJMBPCAUPMBdPzxA6eUB4djK5FxFtFEtjq1EGkUAGjS/d/2zBWW7s/cy/P6T6gMP0Hwuyl8BC5/OOii2PswLUqkq7Wz6B7nXOBlNiFjtAAAAAElFTkSuQmCC
B64

hsx eval "$F" - <<JS
const sht = workbook.getSheet(0);
const T  = GC.Spread.Sheets.Shapes.AutoShapeType;
const BLUE   = "${PNG_BLUE}";
const ORANGE = "${PNG_ORANGE}";
const GREEN  = "${PNG_GREEN}";

// Wide canvas — give every panel a labeled column header.
for (let c = 0; c < 16; c++) sht.setColumnWidth(c, 90);
for (let r = 0; r < 24; r++) sht.setRowHeight(r, 22);

// Labels above each panel so the visual diff is self-describing.
function label(addr, text) {
  sht.setValue.apply(sht, addr.concat([text]));
}
// (We use simple text in A1/A6/A12/A18 etc.)
sht.setValue(0, 0, "A: z-order (XML order = paint order, diamond on top)");
sht.setValue(5, 0, "B: standalone pictures (pictures.add x2 + addPictureShape x1)");
sht.setValue(11, 0, "C: group of 3 shapes (rect + oval + chevron)");
sht.setValue(17, 0, "D: group with nested picture");
sht.setValue(23, 0, "E: nested groups (outer { rect, inner { tri, oval } })");

// ─────────── Panel A: z-order ───────────
// Three overlapping shapes. XML emit order: rect, oval, diamond.
// The diamond must paint LAST (on top). Each is offset by ~50% width.
const a1 = sht.shapes.add("zA-rect",    T.rectangle,  20,  30, 120, 90);
const a2 = sht.shapes.add("zA-oval",    T.oval,       80,  50, 120, 90);
const a3 = sht.shapes.add("zA-diamond", T.diamond,   140,  70, 120, 90);
try { a1.text("1"); a2.text("2"); a3.text("3"); } catch(_) {}

// ─────────── Panel B: standalone pictures ───────────
// Two via pictures.add (the "loose" pic path, anchor-derived geometry),
// one via shapes.addPictureShape (xfrm-driven).
sht.pictures.add("pB-blue",   BLUE,   320, 130, 110, 110);
sht.pictures.add("pB-orange", ORANGE, 450, 130, 110, 110);
sht.shapes.addPictureShape("pB-green", GREEN, 580, 130, 110, 110);

// ─────────── Panel C: simple group of 3 shapes ───────────
const c1 = sht.shapes.add("gC-rect",    T.rectangle, 20, 280, 90, 80);
const c2 = sht.shapes.add("gC-oval",    T.oval,     115, 280, 90, 80);
const c3 = sht.shapes.add("gC-chev",    T.chevron,  210, 280, 90, 80);
sht.shapes.group([c1, c2, c3]);

// ─────────── Panel D: group with nested picture ───────────
// One label rect + one picture, then group them. Tests the
// nested-pic-in-group path that shapes::visit_group special-cases.
const d1 = sht.shapes.add("gD-label",   T.rectangle, 380, 280, 100, 80);
try { d1.text("photo:"); } catch(_) {}
const d2 = sht.shapes.addPictureShape("gD-pic", BLUE, 485, 280, 80, 80);
sht.shapes.group([d1, d2]);

// ─────────── Panel E: nested groups ───────────
// Outer = { rect, inner = { triangle, oval } }
// We build the inner group first, then group the outer rect with the
// already-grouped inner. SpreadJS persists this as <xdr:grpSp> nested
// inside an outer <xdr:grpSp>.
const eOuterRect = sht.shapes.add("gE-outerRect", T.rectangle,  20, 440, 100, 90);
const eInnerTri  = sht.shapes.add("gE-innerTri",  T.isoscelesTriangle, 130, 440, 80, 90);
const eInnerOval = sht.shapes.add("gE-innerOval", T.oval,      215, 440, 80, 90);
const inner = sht.shapes.group([eInnerTri, eInnerOval]);
sht.shapes.group([eOuterRect, inner]);
JS

echo "wrote $F"
