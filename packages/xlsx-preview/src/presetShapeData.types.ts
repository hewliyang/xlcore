export type Token = string;

export interface Formula {
  op: string;
  args: Token[];
}

export interface GuideEntry {
  name: string;
  fmla: Formula;
}

export type PathCmd =
  | { op: "M"; x: Token; y: Token }
  | { op: "L"; x: Token; y: Token }
  | { op: "A"; wR: Token; hR: Token; stAng: Token; swAng: Token }
  | { op: "Q"; x1: Token; y1: Token; x: Token; y: Token }
  | { op: "C"; x1: Token; y1: Token; x2: Token; y2: Token; x: Token; y: Token }
  | { op: "Z" };

export interface PresetPath {
  cmds: PathCmd[];
  w?: number;
  h?: number;
  fill?: string;
  stroke?: false;
  extrusionOk?: false;
}

export interface PresetTextRect {
  l: Token;
  t: Token;
  r: Token;
  b: Token;
}

export interface PresetShape {
  av: GuideEntry[];
  gd: GuideEntry[];
  rect?: PresetTextRect;
  paths: PresetPath[];
}

export type PresetShapeTable = Record<string, PresetShape>;
