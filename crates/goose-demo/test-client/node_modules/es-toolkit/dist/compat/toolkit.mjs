import * as compat from './compat.mjs';

const toolkit = ((value) => {
    return value;
});
Object.assign(toolkit, compat);
toolkit.partial.placeholder = toolkit;
toolkit.partialRight.placeholder = toolkit;

export { toolkit };
