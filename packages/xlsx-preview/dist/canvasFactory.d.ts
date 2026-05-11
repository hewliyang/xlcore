type CanvasLike = {
    width: number;
    height: number;
    getContext(t: "2d"): CanvasRenderingContext2D | null;
};
export declare function setOffscreenCanvasFactory(factory: ((width: number, height: number) => CanvasLike) | null): void;
export declare function makeOffscreenCanvas(width: number, height: number): CanvasLike;
export {};
