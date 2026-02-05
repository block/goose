import React from 'react';
/**
 * Adds one or more newline (\n) characters. Must be used within <Text> components.
 */
export default function Newline({ count = 1 }) {
    return React.createElement("ink-text", null, '\n'.repeat(count));
}
//# sourceMappingURL=Newline.js.map