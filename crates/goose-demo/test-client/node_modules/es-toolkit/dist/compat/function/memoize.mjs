function memoize(func, resolver) {
    if (typeof func !== 'function' || (resolver != null && typeof resolver !== 'function')) {
        throw new TypeError('Expected a function');
    }
    const memoized = function (...args) {
        const key = resolver ? resolver.apply(this, args) : args[0];
        const cache = memoized.cache;
        if (cache.has(key)) {
            return cache.get(key);
        }
        const result = func.apply(this, args);
        memoized.cache = cache.set(key, result) || cache;
        return result;
    };
    const CacheConstructor = memoize.Cache || Map;
    memoized.cache = new CacheConstructor();
    return memoized;
}
memoize.Cache = Map;

export { memoize };
