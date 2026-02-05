declare namespace cliBoxes {
	/**
	Style of the box border.
	*/
	interface BoxStyle {
		readonly topLeft: string;
		readonly top: string;
		readonly topRight: string;
		readonly right: string;
		readonly bottomRight: string;
		readonly bottom: string;
		readonly bottomLeft: string;
		readonly left: string;
	}

	/**
	All box styles.
	*/
	interface Boxes {
		/**
		@example
		```
		┌────┐
		│    │
		└────┘
		```
		*/
		readonly single: BoxStyle;

		/**
		@example
		```
		╔════╗
		║    ║
		╚════╝
		```
		*/
		readonly double: BoxStyle;

		/**
		@example
		```
		╭────╮
		│    │
		╰────╯
		```
		*/
		readonly round: BoxStyle;

		/**
		@example
		```
		┏━━━━┓
		┃    ┃
		┗━━━━┛
		```
		*/
		readonly bold: BoxStyle;

		/**
		@example
		```
		╓────╖
		║    ║
		╙────╜
		```
		*/
		readonly singleDouble: BoxStyle;

		/**
		@example
		```
		╒════╕
		│    │
		╘════╛
		```
		*/
		readonly doubleSingle: BoxStyle;

		/**
		@example
		```
		+----+
		|    |
		+----+
		```
		*/
		readonly classic: BoxStyle;

		/**
		@example
		```
		↘↓↓↓↓↙
		→    ←
		↗↑↑↑↑↖
		```
		*/
		readonly arrow: BoxStyle;
	}
}

/**
Boxes for use in the terminal.

@example
```
import cliBoxes = require('cli-boxes');

console.log(cliBoxes.single);
// {
// 	topLeft: '┌',
// 	top: '─',
// 	topRight: '┐',
// 	right: '│',
// 	bottomRight: '┘',
// 	bottom: '─',
// 	bottomLeft: '└',
// 	left: '│'
// }
```
*/
declare const cliBoxes: cliBoxes.Boxes & {
	// TODO: Remove this for the next major release
	default: typeof cliBoxes;
};

export = cliBoxes;
