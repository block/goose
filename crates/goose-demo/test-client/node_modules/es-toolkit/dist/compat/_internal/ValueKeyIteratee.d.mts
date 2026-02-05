import { IterateeShorthand } from './IterateeShorthand.mjs';

type ValueKeyIteratee<T> = ((value: T, key: string) => unknown) | IterateeShorthand<T>;

export type { ValueKeyIteratee };
