import { IterateeShorthand } from './IterateeShorthand.mjs';

type ValueIterateeCustom<T, TResult> = ((value: T) => TResult) | IterateeShorthand<T>;

export type { ValueIterateeCustom };
