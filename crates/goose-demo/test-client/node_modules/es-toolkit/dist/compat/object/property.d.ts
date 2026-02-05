import { PropertyPath } from '../_internal/PropertyPath.js';

declare function property<T, R>(path: PropertyPath): (obj: T) => R;

export { property };
