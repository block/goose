function endsWith(str, target, position) {
    if (str == null || target == null) {
        return false;
    }
    if (position == null) {
        position = str.length;
    }
    return str.endsWith(target, position);
}

export { endsWith };
