/**
 * twoCellAnchor: from/to cell indices (0-based) + EMU offsets within the cell.
 * 1 EMU = 1/9525 px at 96 DPI.
 */
export type DrawingAnchor = {
    fromCol: number;
    fromColOffEmu: number;
    fromRow: number;
    fromRowOffEmu: number;
    toCol: number;
    toColOffEmu: number;
    toRow: number;
    toRowOffEmu: number;
};
