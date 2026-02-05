# is-in-ci

> Check if the process is running in a [Continuous Integration](https://en.wikipedia.org/wiki/Continuous_integration) (CI) environment

## Install

```sh
npm install is-in-ci
```

## Usage

```js
import isInCi from 'is-in-ci';

if (isInCi) {
	console.log('Running in a CI environment');
}
```

It looks for these environment variables: `CI`, `CONTINUOUS_INTEGRATION`, or any with a `CI_` prefix.

## CLI

```sh
is-in-ci && echo 'Running in a CI environment'
```

Exits with code `0` in CI environments and `1` otherwise.

## FAQ

### How can I add a CI service?

Request the CI service to include the `CI` environment variable. Most already do.

### How is this different from [`is-ci`](https://github.com/watson/is-ci)?

The `is-ci` package attempts to detect every CI service, which is unsustainable. It also has a higher risk of false-positives. For example, it detects the environment variable `RUN_ID` as CI-specific, although other services could use it. Constant updates for new CIs create version fragmentation, resulting in inconsistent behavior across dependent packages. Pushing for CI services to use a standardized CI environment variable is a more robust solution.

## Related

- [is-inside-container](https://github.com/sindresorhus/is-inside-container) - Check if the process is running inside a container
- [is-interactive](https://github.com/sindresorhus/is-interactive) - Check if stdout or stderr is interactive
