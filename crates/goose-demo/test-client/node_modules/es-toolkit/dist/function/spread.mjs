function spread(func) {
    return function (argsArr) {
        return func.apply(this, argsArr);
    };
}

export { spread };
