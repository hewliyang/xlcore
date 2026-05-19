import type { Sparkline } from "./Sparkline.js";

export type SparklineGroup = {
  sparkType: string;

  lineWeight: number;
  markers: boolean;
  high: boolean;
  low: boolean;
  first: boolean;
  last: boolean;
  negative: boolean;

  displayXAxis: boolean;
  rightToLeft: boolean;

  displayEmptyCellsAs: string;

  minAxisType: string;
  maxAxisType: string;

  manualMin?: number;
  manualMax?: number;

  groupMin?: number;
  groupMax?: number;

  colorSeries?: string;
  colorNegative?: string;
  colorAxis?: string;
  colorMarkers?: string;
  colorFirst?: string;
  colorLast?: string;
  colorHigh?: string;
  colorLow?: string;

  sparklines: Array<Sparkline>;
};
