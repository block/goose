import { PartialShallow } from './PartialShallow.mjs';

type IterateeShorthand<T> = PropertyKey | [PropertyKey, any] | PartialShallow<T>;

export type { IterateeShorthand };
