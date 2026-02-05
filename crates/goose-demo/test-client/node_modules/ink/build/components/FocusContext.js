import { createContext } from 'react';
// eslint-disable-next-line @typescript-eslint/naming-convention
const FocusContext = createContext({
    activeId: undefined,
    add() { },
    remove() { },
    activate() { },
    deactivate() { },
    enableFocus() { },
    disableFocus() { },
    focusNext() { },
    focusPrevious() { },
    focus() { },
});
FocusContext.displayName = 'InternalFocusContext';
export default FocusContext;
//# sourceMappingURL=FocusContext.js.map