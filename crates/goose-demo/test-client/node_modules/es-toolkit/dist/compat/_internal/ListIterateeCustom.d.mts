import { PartialShallow } from './PartialShallow.mjs';

type ListIterateeCustom<T, R> = ((value: T, index: number, collection: ArrayLike<T>) => R) | (PropertyKey | [PropertyKey, any] | PartialShallow<T>);

export type { ListIterateeCustom };
