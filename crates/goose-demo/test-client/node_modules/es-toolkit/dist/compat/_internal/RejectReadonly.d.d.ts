import { IsWritable } from './IsWritable.d.js';
import { MutableList } from './MutableList.d.js';

type RejectReadonly<T extends MutableList<unknown>> = IsWritable<T> extends true ? T : never;

export type { RejectReadonly };
