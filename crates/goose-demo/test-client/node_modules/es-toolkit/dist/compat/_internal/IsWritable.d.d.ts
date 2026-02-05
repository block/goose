import { Equals } from './Equals.d.js';

type IsWritable<T> = Equals<{ [K in keyof T]: T[K] }, { -readonly [K in keyof T]: T[K] }>;

export type { IsWritable };
