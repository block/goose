# @alcalzone/ansi-tokenize

> Efficiently modify strings containing [ANSI escape codes](https://en.wikipedia.org/wiki/ANSI_escape_code#Colors_and_Styles)

If you find yourself modifying styled strings repeatedly, alternatives like [`slice-ansi`](https://github.com/chalk/slice-ansi/) may end up doing a lot of unnecessary work by re-parsing the string each time. This module provides a way to parse the string into an array of tokens (characters or ANSI codes), which can then be modified and re-serialized into a styled string.

## Install

```
$ npm install @alcalzone/ansi-tokenize
```

## Usage

### Tokenize a string

```js
import { tokenize } from "@alcalzone/ansi-tokenize";

// red "foo", followed by unstyled "bar"
const str = "\x1B[31mfoo\x1B[39mbar";
const tokens = tokenize(str);

// tokens will now look like this:
[
	{
		type: "ansi",
		code: "\x1B[31m",
		endCode: "\x1B[39m",
	},
	{
		type: "char",
		value: "f",
		fullWidth: false,
	},
	{
		type: "char",
		value: "o",
		fullWidth: false,
	},
	{
		type: "char",
		value: "o",
		fullWidth: false,
	},
	{
		type: "ansi",
		code: "\x1B[39m",
		endCode: "\x1B[39m",
	},
	{
		type: "char",
		value: "b",
		fullWidth: false,
	},
	{
		type: "char",
		value: "a",
		fullWidth: false,
	},
	{
		type: "char",
		value: "r",
		fullWidth: false,
	},
];
```

Each token is either a character

```ts
export interface Char {
	type: "char";
	value: string;
	fullWidth: boolean;
}
```

where

-   `value` is the string representation of the character
-   `fullWidth` is `true` if the character is full width (takes up 2 characters in monospace, like CJK characters)

or an ANSI code

```ts
export interface AnsiCode {
	type: "ansi";
	code: string;
	endCode: string;
}
```

where

-   `code` is the ANSI code that starts the style
-   and `endCode` is the corresponding ANSI code that ends the style.

An `AnsiCode` can also be an end code, in which case `code` and `endCode` will be the same.

### Convert an array of tokens into an array of "styled" chars

This representation is a 1:1 mapping of the original string, but not very useful for modifying the string. The `styledCharsFromTokens` function converts a token array to an array of characters, where each character has an all currently active styles associated with it:

```ts
export interface StyledChar {
	type: "char";
	value: string;
	fullWidth: boolean;
	styles: AnsiCode[];
}
```

Using the above example:

```js
import { tokenize, styledCharsFromTokens } from "@alcalzone/ansi-tokenize";

// red "foo", followed by unstyled "bar"
const str = "\x1B[31mfoo\x1B[39mbar";
const tokens = tokenize(str);

const styledChars = styledCharsFromTokens(tokens);

// styledChars will contain the following:
[
	{
		type: "char",
		value: "f",
		fullWidth: false,
		styles: [
			{
				type: "ansi",
				code: "\x1B[31m",
				endCode: "\x1B[39m",
			},
		],
	},
	{
		type: "char",
		value: "o",
		fullWidth: false,
		styles: [
			{
				type: "ansi",
				code: "\x1B[31m",
				endCode: "\x1B[39m",
			},
		],
	},
	{
		type: "char",
		value: "o",
		fullWidth: false,
		styles: [
			{
				type: "ansi",
				code: "\x1B[31m",
				endCode: "\x1B[39m",
			},
		],
	},
	{
		type: "char",
		value: "b",
		fullWidth: false,
		styles: [],
	},
	{
		type: "char",
		value: "a",
		fullWidth: false,
		styles: [],
	},
	{
		type: "char",
		value: "r",
		fullWidth: false,
		styles: [],
	},
];
```

### Modify an array of styled characters

For modification simply edit the items in the array as necessary, as long as the following rules are followed:

1. The `code` and `endCode` properties must match. You can use the `ansi-styles` module to do this.
2. The `fullWidth` property must be correct. You can use the `is-fullwidth-code-point` module to do this, or if working with multiple strings, turn those into styled char arrays first.

E.g. to make the first `o` blue and bold:

```js
import ansiStyles from "ansi-styles";

// ... include the above code

styledChars[1].styles = [
	{
		type: "ansi",
		code: ansiStyles.blue.open,
		endCode: ansiStyles.blue.close,
	},
	{
		type: "ansi",
		code: ansiStyles.bold.open,
		endCode: ansiStyles.bold.close,
	},
];
```

### Serialize a styled character array back to a string

The `styledCharsToString` function converts a styled character array back to a string:

```js
import { styledCharsToString } from "@alcalzone/ansi-tokenize";

// ... include the above code

const strOut = styledCharsToString(styledChars);

// str will now be '\x1B[31mf\x1B[34m\x1B[1mo\x1B[22m\x1B[31mo\x1B[39mbar'
```

This automatically figures out the least amount of escape codes necessary to achieve the desired result, as long as the `styles` arrays contain no unnecessary styles, e.g. blue + red foreground.

## Changelog

<!--
	Placeholder for next release:
	### __WORK IN PROGRESS__
-->
### 0.1.3 (2023-09-07)

-   Fix: Support links

### 0.1.2 (2023-08-07)

-   Fix: Reduce minimum Node.js version to `14.13.1`

### 0.1.1 (2023-04-05)

-   Fix: Active styles are now correctly reset at the end of the string

### 0.1.0 (2023-03-20)

Initial release
