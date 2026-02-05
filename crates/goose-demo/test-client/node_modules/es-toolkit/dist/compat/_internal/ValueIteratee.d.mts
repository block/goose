import { PartialShallow } from './PartialShallow.mjs';

type ValueIteratee<T> = ((value: T) => unknown) | (PropertyKey | [PropertyKey, any] | PartialShallow<T>);

export type { ValueIteratee };
