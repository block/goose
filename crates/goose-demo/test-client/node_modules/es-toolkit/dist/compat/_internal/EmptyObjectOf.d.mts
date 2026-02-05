type EmptyObject<T> = {
    [K in keyof T]?: never;
};
type EmptyObjectOf<T> = EmptyObject<T> extends T ? EmptyObject<T> : never;

export type { EmptyObjectOf };
