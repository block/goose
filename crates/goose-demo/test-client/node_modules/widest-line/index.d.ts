/**
Get the visual width of the widest line in a string - the number of columns required to display it.

@example
```
import widestLine from 'widest-line';

widestLine('古\n\u001B[1m@\u001B[22m');
//=> 2
```
*/
export default function widestLine(string: string): number;
