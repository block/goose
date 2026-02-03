import re
from re import Pattern

_GLOB_CACHE: dict[str, Pattern[str]] = {}


def compile_glob(pattern: str) -> Pattern[str]:
    """Convert a glob pattern supporting ** into a compiled regex.

    Args:
        pattern: The glob pattern string.

    Returns:
        A compiled regex pattern object.
    """
    cached = _GLOB_CACHE.get(pattern)
    if cached:
        return cached

    regex_parts: list[str] = []
    i = 0
    length = len(pattern)
    while i < length:
        char = pattern[i]
        if char == "*":
            if i + 1 < length and pattern[i + 1] == "*":
                regex_parts.append(".*")
                i += 1
            else:
                regex_parts.append("[^/]*")
        elif char == "?":
            regex_parts.append("[^/]")
        else:
            regex_parts.append(re.escape(char))
        i += 1

    compiled = re.compile("^" + "".join(regex_parts) + "$")
    _GLOB_CACHE[pattern] = compiled
    return compiled


def expand_pattern_variants(pattern: str) -> set[str]:
    """Generate fallback globs so ** can match zero directories.

    Args:
        pattern: The glob pattern to expand.

    Returns:
        A set of pattern variants.
    """
    variants = {pattern}
    queue = [pattern]

    while queue:
        current = queue.pop()
        normalized = current.replace("//", "/")

        transformations = [
            ("/**/", "/"),
            ("**/", ""),
            ("/**", ""),
            ("**", ""),
        ]

        for old, new in transformations:
            if old in normalized:
                replaced = normalized.replace(old, new, 1)
                replaced = replaced.replace("//", "/")
                if replaced not in variants:
                    variants.add(replaced)
                    queue.append(replaced)

    return variants


def matches_any(path: str, patterns: list[str]) -> bool:
    """Check if a path matches any of the given patterns.

    Args:
        path: The file path to check.
        patterns: A list of glob patterns.

    Returns:
        True if the path matches any pattern, False otherwise.
    """
    if not path or not patterns:
        return False

    normalized_path = path.replace("\\", "/")
    for pattern in patterns:
        for variant in expand_pattern_variants(pattern.replace("\\", "/")):
            compiled = compile_glob(variant)
            if compiled.match(normalized_path):
                return True
    return False
