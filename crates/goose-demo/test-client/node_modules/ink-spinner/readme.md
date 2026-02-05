# ink-spinner ![test](https://github.com/vadimdemedes/ink-spinner/workflows/test/badge.svg)

> Spinner component for [Ink](https://github.com/vadimdemedes/ink). Uses [cli-spinners](https://github.com/sindresorhus/cli-spinners) for the collection of spinners.

Looking for a version compatible with Ink 2.x? Check out [this release](https://github.com/vadimdemedes/ink-spinner/tree/v3.1.0).

## Install

```
$ npm install ink-spinner
```

## Usage

```jsx
import React from 'react';
import { render, Text } from 'ink';
import Spinner from 'ink-spinner';

render(
	<Text>
		<Text color="green">
			<Spinner type="dots" />
		</Text>
		{' Loading'}
	</Text>
);
```

<img src="media/demo.gif" width="482">

## Props

### type

Type: `string`<br>
Defaults: `dots`

Type of a spinner. See [cli-spinners](https://github.com/sindresorhus/cli-spinners) for available spinners.
