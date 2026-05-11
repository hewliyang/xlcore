import type { Sheet as WireSheet } from "./schema/Sheet.js";
import type { DecodedCells, DecodedRowMeta } from "./columnar.js";
export interface Sheet extends WireSheet {
    decodedCells: DecodedCells;
    decodedRowMeta: DecodedRowMeta;
}
export type { Border, BorderLine, Cell, CellFormat, CellRef, CfColorScale, CfColorScaleStop, CfDataBar, CfIconSet, CfRule, CfvoStop, Chart, ChartSeries, DataLabels, Col, Color, Comment, ConditionalFormat, Drawing, DrawingAnchor, Dxf, Fill, Font, GradientStop, Freeze, Hyperlink, Image, Merge, NumberFormat, OutlinePr, Pivot, Sparkline, SparklineGroup, Styles, Table, TableColumn, TableStyle, TextRun, Theme, WorkbookLayout, } from "./schema/index.js";
