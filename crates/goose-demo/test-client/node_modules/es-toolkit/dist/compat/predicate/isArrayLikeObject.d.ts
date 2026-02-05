declare function isArrayLikeObject<T extends {
    __lodashAnyHack: any;
}>(value: T): boolean;
declare function isArrayLikeObject(value: ((...args: any[]) => any) | Function | string | boolean | number | null | undefined): value is never;
declare function isArrayLikeObject(value: any): value is object & {
    length: number;
};

export { isArrayLikeObject };
