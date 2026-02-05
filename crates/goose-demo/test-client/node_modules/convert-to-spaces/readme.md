# convert-to-spaces [![Build Status](https://github.com/vadimdemedes/convert-to-spaces/workflows/test/badge.svg)](https://github.com/vadimdemedes/convert-to-spaces/actions)

> Convert tabs to spaces in a string

## Install

```
$ npm install --save convert-to-spaces
```

## Usage

```js
import convertToSpaces from 'convert-to-spaces';

convertToSpaces('\t\thello!');
//=> '    hello!'
```

## API

### convertToSpaces(input, [spaces])

#### input

Type: `string`

String to convert.

#### spaces

Type: `number`<br>
Default: `2`

Number of spaces instead of each tab.

## Related

- [convert-to-tabs](https://github.com/vadimdemedes/convert-to-tabs) - Convert spaces to tabs.
