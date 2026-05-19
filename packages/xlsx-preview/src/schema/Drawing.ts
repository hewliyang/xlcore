import type { Chart } from "./Chart.js";
import type { DrawingAnchor } from "./DrawingAnchor.js";
import type { Image } from "./Image.js";
import type { Shape } from "./Shape.js";

export type Drawing = {
  kind: string;
  anchor: DrawingAnchor;
  chart?: Chart;
  image?: Image;
  shape?: Shape;
};
