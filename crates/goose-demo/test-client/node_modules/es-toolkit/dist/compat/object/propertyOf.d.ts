import { PropertyPath } from '../_internal/PropertyPath.js';

declare function propertyOf<T extends {}>(object: T): (path: PropertyPath) => any;

export { propertyOf };
