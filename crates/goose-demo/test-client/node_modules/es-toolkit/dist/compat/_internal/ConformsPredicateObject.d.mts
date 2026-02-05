type ConformsPredicateObject<T> = {
    [P in keyof T]: T[P] extends (arg: infer A) => any ? A : any;
};

export type { ConformsPredicateObject };
