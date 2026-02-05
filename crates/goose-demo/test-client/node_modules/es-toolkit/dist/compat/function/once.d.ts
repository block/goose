declare function once<T extends (...args: any) => any>(func: T): T;

export { once };
