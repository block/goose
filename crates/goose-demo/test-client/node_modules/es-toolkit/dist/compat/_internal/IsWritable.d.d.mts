import { Equals } from './Equals.d.mjs';

type IsWritable<T> = Equals<{ [K in keyof T]: T[K] }, { -readonly [K in keyof T]: T[K] }>;

export type { IsWritable };
