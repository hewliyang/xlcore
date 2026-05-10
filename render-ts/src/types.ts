// Re-export of the auto-generated `WorkbookLayout` schema. The source of
// truth is `crates/xlcore-export/src/schema.rs`; ts-rs emits one file per
// type into `./schema/` (regenerate with
// `cargo test --release -p xlcore-export export_bindings`).
//
// We keep this barrel because most renderer code already imports from
// `./types.js` and we don't want a churning rename across the codebase
// every time a schema type is added or removed.
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
  Col,
  Color,
  Comment,
  ConditionalFormat,
  Drawing,
  DrawingAnchor,
  Dxf,
  Fill,
  Font,
  Freeze,
  Hyperlink,
  Image,
  Merge,
  NumberFormat,
  Pivot,
  Row,
  Sheet,
  Styles,
  Table,
  TableColumn,
  TableStyle,
  TextRun,
  Theme,
  WorkbookLayout,
} from "./schema/index.js";
