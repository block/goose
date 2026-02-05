import { IterateeShorthand } from './IterateeShorthand.js';

type ValueKeyIteratee<T> = ((value: T, key: string) => unknown) | IterateeShorthand<T>;

export type { ValueKeyIteratee };
