import type { Equals } from './Equals.d.ts';

export type IsWritable<T> = Equals<{ [K in keyof T]: T[K] }, { -readonly [K in keyof T]: T[K] }>;