export interface Options {
	/**
	Bind only the given methods.
	*/
	readonly include?: ReadonlyArray<string | RegExp>;

	/**
	Bind methods except for the given methods.
	*/
	readonly exclude?: ReadonlyArray<string | RegExp>;
}

/**
Automatically bind methods to their class instance.

@param self - An object with methods to bind.

@example
```
import autoBind from 'auto-bind';

class Unicorn {
	constructor(name) {
		this.name = name;
		autoBind(this);
	}

	message() {
		return `${this.name} is awesome!`;
	}
}

const unicorn = new Unicorn('Rainbow');

// Grab the method off the class instance
const message = unicorn.message;

// Still bound to the class instance
message();
//=> 'Rainbow is awesome!'

// Without `autoBind(this)`, the above would have resulted in
message();
//=> Error: Cannot read property 'name' of undefined
```
*/
export default function autoBind<SelfType extends Record<string, any>>( // This has to use `any` to be compatible with classes.
	self: SelfType,
	options?: Options
): SelfType;
