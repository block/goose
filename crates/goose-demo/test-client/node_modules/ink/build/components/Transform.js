import React from 'react';
/**
 * Transform a string representation of React components before they are written to output.
 * For example, you might want to apply a gradient to text, add a clickable link or create some text effects.
 * These use cases can't accept React nodes as input, they are expecting a string.
 * That's what <Transform> component does, it gives you an output string of its child components and lets you transform it in any way.
 */
export default function Transform({ children, transform }) {
    if (children === undefined || children === null) {
        return null;
    }
    return (React.createElement("ink-text", { style: { flexGrow: 0, flexShrink: 1, flexDirection: 'row' }, internal_transform: transform }, children));
}
//# sourceMappingURL=Transform.js.map