import { PropertyPath } from '../_internal/PropertyPath.mjs';

declare function propertyOf<T extends {}>(object: T): (path: PropertyPath) => any;

export { propertyOf };
