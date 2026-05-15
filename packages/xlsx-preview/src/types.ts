// Re-export of the auto-generated `WorkbookLayout` schema. The source of
// truth is `crates/xlcore-export/src/schema.rs`; ts-rs emits one file per
// type into `./schema/` (regenerate with
// `cargo test --release -p xlcore-export export_bindings`).
//
// We keep this barrel because most renderer code already imports from
// `./types.js` and we don't want a churning rename across the codebase
// every time a schema type is added or removed.

// `Sheet` is special: the wire shape comes from ts-rs, but the runtime
// instance carries decoded typed-array fields (see `./columnar.ts`).
// We can't `interface`-merge a ts-rs `type` alias, so we extend it
// with a runtime-only interface here. Renderer code that imports
// `Sheet` from `./types.js` picks up the augmented shape automatically.
import type { Sheet as WireSheet } from "./schema/Sheet.js";
import type { DecodedCells, DecodedRowMeta } from "./columnar.js";
export interface Sheet extends WireSheet {
  decodedCells: DecodedCells;
  decodedRowMeta: DecodedRowMeta;
}

export type {
  Border,
  BorderLine,
  Cell,
  CellFormat,
  CellRef,
  CfColorScale,
  CfColorScaleStop,
  CfDataBar,
  CfIconSet,
  CfRule,
  CfvoStop,
  Chart,
  ChartSeries,
  DataLabels,
  PointDataLabel,
  Col,
  Color,
  Comment,
  ConditionalFormat,
  Drawing,
  DrawingAnchor,
  Dxf,
  Fill,
  Font,
  GradientStop,
  Freeze,
  Hyperlink,
  Image,
  Merge,
  NumberFormat,
  OutlinePr,
  Pivot,
  Sparkline,
  SparklineGroup,
  Styles,
  Table,
  TableColumn,
  TableStyle,
  TextRun,
  Theme,
  WorkbookLayout,
} from "./schema/index.js";
