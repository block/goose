import { IterateeShorthand } from './IterateeShorthand.mjs';
import { ObjectIterator } from './ObjectIterator.mjs';

type ObjectIteratee<TObject> = ObjectIterator<TObject, unknown> | IterateeShorthand<TObject[keyof TObject]>;
type ObjectIterateeCustom<TObject, TResult> = ObjectIterator<TObject, TResult> | IterateeShorthand<TObject[keyof TObject]>;

export type { ObjectIteratee, ObjectIterateeCustom };
