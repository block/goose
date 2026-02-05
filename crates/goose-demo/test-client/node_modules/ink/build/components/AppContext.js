import { createContext } from 'react';
/**
 * `AppContext` is a React context, which exposes a method to manually exit the app (unmount).
 */
// eslint-disable-next-line @typescript-eslint/naming-convention
const AppContext = createContext({
    exit() { },
});
AppContext.displayName = 'InternalAppContext';
export default AppContext;
//# sourceMappingURL=AppContext.js.map