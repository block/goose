export interface MutableList<T> {
  length: number;
  [k: number]: T;
}
