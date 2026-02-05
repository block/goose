function isBrowser() {
    return typeof window !== 'undefined' && window?.document != null;
}

export { isBrowser };
