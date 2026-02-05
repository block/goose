interface MutableList<T> {
  length: number;
  [k: number]: T;
}

export type { MutableList };
