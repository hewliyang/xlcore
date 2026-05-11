import type { Color, WorkbookLayout } from "./types.js";
export declare function setActiveTheme(theme: WorkbookLayout["theme"]): void;
export declare function activeThemeColor(index: number, fallback: string): string;
export declare function applyTint(hex: string, tint: number): string;
export declare function colorToCss(c: Color | undefined, fallback?: string): string;
