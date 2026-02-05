type Equals<T, U> = (<X>() => X extends T ? 1 : 2) extends <X>() => X extends U ? 1 : 2 ? true : false;

export type { Equals };
