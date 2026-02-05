import { IsWritable } from './IsWritable.d.ts';
import { MutableList } from './MutableList.d.ts';

export type RejectReadonly<T extends MutableList<unknown>> = IsWritable<T> extends true ? T : never;