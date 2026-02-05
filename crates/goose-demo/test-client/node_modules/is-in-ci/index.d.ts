/**
Check if the process is running in a Continuous Integration (CI) environment.

@example
```
import isInCi from 'is-in-ci';

if (isInCi) {
	console.log('Running in a CI environment');
}
```
*/
declare const isInCi: boolean;

export default isInCi;
