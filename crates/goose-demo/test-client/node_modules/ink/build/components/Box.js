import React, { forwardRef } from 'react';
/**
 * `<Box>` is an essential Ink component to build your layout. It's like `<div style="display: flex">` in the browser.
 */
const Box = forwardRef(({ children, ...style }, ref) => {
    return (React.createElement("ink-box", { ref: ref, style: {
            ...style,
            overflowX: style.overflowX ?? style.overflow ?? 'visible',
            overflowY: style.overflowY ?? style.overflow ?? 'visible',
        } }, children));
});
Box.displayName = 'Box';
Box.defaultProps = {
    flexWrap: 'nowrap',
    flexDirection: 'row',
    flexGrow: 0,
    flexShrink: 1,
};
export default Box;
//# sourceMappingURL=Box.js.map