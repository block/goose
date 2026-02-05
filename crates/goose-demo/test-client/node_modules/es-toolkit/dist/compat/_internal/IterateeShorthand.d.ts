import { PartialShallow } from './PartialShallow.js';

type IterateeShorthand<T> = PropertyKey | [PropertyKey, any] | PartialShallow<T>;

export type { IterateeShorthand };
