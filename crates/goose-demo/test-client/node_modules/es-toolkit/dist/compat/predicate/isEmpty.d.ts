import { EmptyObjectOf } from '../_internal/EmptyObjectOf.js';

declare function isEmpty<T extends {
    __trapAny: any;
}>(value?: T): boolean;
declare function isEmpty(value: string): value is '';
declare function isEmpty(value: Map<any, any> | Set<any> | ArrayLike<any> | null | undefined): boolean;
declare function isEmpty(value: object): boolean;
declare function isEmpty<T extends object>(value: T | null | undefined): value is EmptyObjectOf<T> | null | undefined;
declare function isEmpty(value?: any): boolean;

export { isEmpty };
