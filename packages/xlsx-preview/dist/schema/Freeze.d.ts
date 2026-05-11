export type Freeze = {
    /**
     * 1-based: rows above this index are frozen.
     */
    topRow: number;
    /**
     * 1-based: cols left of this index are frozen.
     */
    leftCol: number;
};
