'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const CASE_SPLIT_PATTERN = /\p{Lu}?\p{Ll}+|[0-9]+|\p{Lu}+(?!\p{Ll})|\p{Emoji_Presentation}|\p{Extended_Pictographic}|\p{L}+/gu;
function words(str) {
    return Array.from(str.match(CASE_SPLIT_PATTERN) ?? []);
}

exports.CASE_SPLIT_PATTERN = CASE_SPLIT_PATTERN;
exports.words = words;
