import autoBind from './index.js';

const excludedReactMethods = [
	'componentWillMount',
	'UNSAFE_componentWillMount',
	'render',
	'getSnapshotBeforeUpdate',
	'componentDidMount',
	'componentWillReceiveProps',
	'UNSAFE_componentWillReceiveProps',
	'shouldComponentUpdate',
	'componentWillUpdate',
	'UNSAFE_componentWillUpdate',
	'componentDidUpdate',
	'componentWillUnmount',
	'componentDidCatch',
	'setState',
	'forceUpdate',
];

export default function autoBindReact(self, {exclude = [], ...options} = {}) {
	options.exclude = [
		...exclude,
		...excludedReactMethods,
	];

	return autoBind(self, options);
}
