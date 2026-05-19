import type { PointDataLabel } from "./PointDataLabel.js";

export type DataLabels = {
  showValue: boolean;
  showCategory: boolean;
  showSeriesName: boolean;
  showPercent: boolean;
  position?: string;
  separator?: string;
  numFmt?: string;
  pointOverrides: Array<PointDataLabel>;
};
