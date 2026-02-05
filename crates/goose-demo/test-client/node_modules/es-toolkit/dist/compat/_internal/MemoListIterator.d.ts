type MemoListIterator<T, R, A> = (prev: R, curr: T, index: number, list: A) => R;

export type { MemoListIterator };
