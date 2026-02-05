function startsWith(str, target, position) {
    if (str == null || target == null) {
        return false;
    }
    if (position == null) {
        position = 0;
    }
    return str.startsWith(target, position);
}

export { startsWith };
