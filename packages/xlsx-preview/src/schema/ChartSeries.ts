import type { DataLabels } from "./DataLabels.js";

export type ChartSeries = {
  name: string;

  nameRef?: string;

  color?: string;
  values: Array<number>;

  valuesRef?: string;

  xValues: Array<number>;

  xValuesRef?: string;
  bubbleSizes: Array<number>;
  bubbleSizesRef?: string;

  pointColors: Array<string>;

  dataLabels?: DataLabels;
  axisGroup?: string;
  chartType?: string;
  markerSymbol?: string;
};
