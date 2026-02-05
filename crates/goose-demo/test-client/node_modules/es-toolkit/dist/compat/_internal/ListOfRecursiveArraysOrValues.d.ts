import { RecursiveArray } from './RecursiveArray.js';

interface ListOfRecursiveArraysOrValues<T> extends ArrayLike<T | RecursiveArray<T>> {
}

export type { ListOfRecursiveArraysOrValues };
