import {Component as ReactComponent} from 'react';
import autoBind, {Options} from './index.js';

/**
Same as `autoBind` but excludes the default [React component methods](https://reactjs.org/docs/react-component.html).

@param self - An object with methods to bind.

@example
```
import autoBindReact from 'auto-bind/react';

class Foo extends React.Component {
	constructor(props) {
		super(props);
		autoBindReact(this);
	}

	// …
}
```
*/
export default function autoBindReact<SelfType extends ReactComponent>(
	self: SelfType,
	options?: Options
): SelfType;
