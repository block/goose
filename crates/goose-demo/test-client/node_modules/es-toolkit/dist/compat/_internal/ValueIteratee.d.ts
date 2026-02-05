import { PartialShallow } from './PartialShallow.js';

type ValueIteratee<T> = ((value: T) => unknown) | (PropertyKey | [PropertyKey, any] | PartialShallow<T>);

export type { ValueIteratee };
