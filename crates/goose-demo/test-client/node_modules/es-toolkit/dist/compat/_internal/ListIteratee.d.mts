import { PartialShallow } from './PartialShallow.mjs';

type ListIteratee<T> = ((value: T, index: number, collection: ArrayLike<T>) => unknown) | (PropertyKey | [PropertyKey, any] | PartialShallow<T>);

export type { ListIteratee };
