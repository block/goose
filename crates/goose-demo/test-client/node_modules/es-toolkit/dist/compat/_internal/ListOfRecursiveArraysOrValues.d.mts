import { RecursiveArray } from './RecursiveArray.mjs';

interface ListOfRecursiveArraysOrValues<T> extends ArrayLike<T | RecursiveArray<T>> {
}

export type { ListOfRecursiveArraysOrValues };
