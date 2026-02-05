const deburrMap = new Map([
    ['Æ', 'Ae'],
    ['Ð', 'D'],
    ['Ø', 'O'],
    ['Þ', 'Th'],
    ['ß', 'ss'],
    ['æ', 'ae'],
    ['ð', 'd'],
    ['ø', 'o'],
    ['þ', 'th'],
    ['Đ', 'D'],
    ['đ', 'd'],
    ['Ħ', 'H'],
    ['ħ', 'h'],
    ['ı', 'i'],
    ['Ĳ', 'IJ'],
    ['ĳ', 'ij'],
    ['ĸ', 'k'],
    ['Ŀ', 'L'],
    ['ŀ', 'l'],
    ['Ł', 'L'],
    ['ł', 'l'],
    ['ŉ', "'n"],
    ['Ŋ', 'N'],
    ['ŋ', 'n'],
    ['Œ', 'Oe'],
    ['œ', 'oe'],
    ['Ŧ', 'T'],
    ['ŧ', 't'],
    ['ſ', 's'],
]);
function deburr(str) {
    str = str.normalize('NFD');
    let result = '';
    for (let i = 0; i < str.length; i++) {
        const char = str[i];
        if ((char >= '\u0300' && char <= '\u036f') || (char >= '\ufe20' && char <= '\ufe23')) {
            continue;
        }
        result += deburrMap.get(char) ?? char;
    }
    return result;
}

export { deburr };
