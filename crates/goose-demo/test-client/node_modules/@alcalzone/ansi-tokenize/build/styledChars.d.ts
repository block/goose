import type { AnsiCode, Char, Token } from "./tokenize.js";
export interface StyledChar extends Char {
    styles: AnsiCode[];
}
export declare function styledCharsFromTokens(tokens: Token[]): StyledChar[];
export declare function styledCharsToString(chars: StyledChar[]): string;
