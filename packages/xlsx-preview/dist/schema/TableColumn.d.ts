export type TableColumn = {
    name: string;
    /**
     * `sum`, `average`, `count`, `countNums`, `min`, `max`, `stdDev`,
     * `var`, `custom`. None ⇒ no totals function for this column.
     */
    totalsRowFunction?: string;
    /**
     * Literal label shown in the totals row (e.g. "Total").
     */
    totalsRowLabel?: string;
};
