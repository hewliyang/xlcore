export type CellFormat = {
    fontId?: number;
    fillId?: number;
    borderId?: number;
    numFmtId?: number;
    /**
     * "left","center","right","general","fill","justify" (lower-case).
     */
    horizontalAlignment?: string;
    /**
     * "top","center","bottom".
     */
    verticalAlignment?: string;
    wrapText: boolean;
    indent?: number;
    textRotation?: number;
};
