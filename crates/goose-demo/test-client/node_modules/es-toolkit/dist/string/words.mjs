const CASE_SPLIT_PATTERN = /\p{Lu}?\p{Ll}+|[0-9]+|\p{Lu}+(?!\p{Ll})|\p{Emoji_Presentation}|\p{Extended_Pictographic}|\p{L}+/gu;
function words(str) {
    return Array.from(str.match(CASE_SPLIT_PATTERN) ?? []);
}

export { CASE_SPLIT_PATTERN, words };
