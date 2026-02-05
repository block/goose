import { PartialShallow } from './PartialShallow.js';

type ListIterateeCustom<T, R> = ((value: T, index: number, collection: ArrayLike<T>) => R) | (PropertyKey | [PropertyKey, any] | PartialShallow<T>);

export type { ListIterateeCustom };
