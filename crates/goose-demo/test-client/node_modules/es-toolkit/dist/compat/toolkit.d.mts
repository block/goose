import * as compat from './compat.mjs';

type ToolkitFn = (value: any) => any;
type Compat = typeof compat;
interface Toolkit extends ToolkitFn, Compat {
}
declare const toolkit: Toolkit;

export { type Toolkit, toolkit };
