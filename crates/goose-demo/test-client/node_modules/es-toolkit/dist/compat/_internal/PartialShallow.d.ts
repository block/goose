type PartialShallow<T> = {
    [P in keyof T]?: T[P] extends object ? object : T[P];
};

export type { PartialShallow };
