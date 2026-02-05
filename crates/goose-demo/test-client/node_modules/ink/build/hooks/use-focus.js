import { useEffect, useContext, useMemo } from 'react';
import FocusContext from '../components/FocusContext.js';
import useStdin from './use-stdin.js';
/**
 * Component that uses `useFocus` hook becomes "focusable" to Ink,
 * so when user presses <kbd>Tab</kbd>, Ink will switch focus to this component.
 * If there are multiple components that execute `useFocus` hook, focus will be
 * given to them in the order that these components are rendered in.
 * This hook returns an object with `isFocused` boolean property, which
 * determines if this component is focused or not.
 */
const useFocus = ({ isActive = true, autoFocus = false, id: customId, } = {}) => {
    const { isRawModeSupported, setRawMode } = useStdin();
    const { activeId, add, remove, activate, deactivate, focus } = useContext(FocusContext);
    const id = useMemo(() => {
        return customId ?? Math.random().toString().slice(2, 7);
    }, [customId]);
    useEffect(() => {
        add(id, { autoFocus });
        return () => {
            remove(id);
        };
    }, [id, autoFocus]);
    useEffect(() => {
        if (isActive) {
            activate(id);
        }
        else {
            deactivate(id);
        }
    }, [isActive, id]);
    useEffect(() => {
        if (!isRawModeSupported || !isActive) {
            return;
        }
        setRawMode(true);
        return () => {
            setRawMode(false);
        };
    }, [isActive]);
    return {
        isFocused: Boolean(id) && activeId === id,
        focus,
    };
};
export default useFocus;
//# sourceMappingURL=use-focus.js.map