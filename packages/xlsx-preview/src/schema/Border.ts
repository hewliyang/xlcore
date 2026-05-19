import type { BorderLine } from "./BorderLine.js";

export type Border = {
  left?: BorderLine;
  right?: BorderLine;
  top?: BorderLine;
  bottom?: BorderLine;

  diagonalUp: boolean;

  diagonalDown: boolean;

  diagonal?: BorderLine;
};
