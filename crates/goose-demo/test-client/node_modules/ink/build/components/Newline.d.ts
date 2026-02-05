import React from 'react';
export type Props = {
    /**
     * Number of newlines to insert.
     *
     * @default 1
     */
    readonly count?: number;
};
/**
 * Adds one or more newline (\n) characters. Must be used within <Text> components.
 */
export default function Newline({ count }: Props): React.JSX.Element;
