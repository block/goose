import { useContext } from 'react';
import FocusContext from '../components/FocusContext.js';
/**
 * This hook exposes methods to enable or disable focus management for all
 * components or manually switch focus to next or previous components.
 */
const useFocusManager = () => {
    const focusContext = useContext(FocusContext);
    return {
        enableFocus: focusContext.enableFocus,
        disableFocus: focusContext.disableFocus,
        focusNext: focusContext.focusNext,
        focusPrevious: focusContext.focusPrevious,
        focus: focusContext.focus,
    };
};
export default useFocusManager;
//# sourceMappingURL=use-focus-manager.js.map