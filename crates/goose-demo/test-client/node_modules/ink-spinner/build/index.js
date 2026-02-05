import React, { useState, useEffect } from 'react';
import { Text } from 'ink';
import spinners from 'cli-spinners';
/**
 * Spinner.
 */
function Spinner({ type = 'dots' }) {
    const [frame, setFrame] = useState(0);
    const spinner = spinners[type];
    useEffect(() => {
        const timer = setInterval(() => {
            setFrame(previousFrame => {
                const isLastFrame = previousFrame === spinner.frames.length - 1;
                return isLastFrame ? 0 : previousFrame + 1;
            });
        }, spinner.interval);
        return () => {
            clearInterval(timer);
        };
    }, [spinner]);
    return React.createElement(Text, null, spinner.frames[frame]);
}
export default Spinner;
//# sourceMappingURL=index.js.map