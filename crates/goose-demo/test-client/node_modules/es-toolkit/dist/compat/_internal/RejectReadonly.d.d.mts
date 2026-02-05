import { IsWritable } from './IsWritable.d.mjs';
import { MutableList } from './MutableList.d.mjs';

type RejectReadonly<T extends MutableList<unknown>> = IsWritable<T> extends true ? T : never;

export type { RejectReadonly };
