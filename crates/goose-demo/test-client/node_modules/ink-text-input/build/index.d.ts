import React from 'react';
import type { Except } from 'type-fest';
export type Props = {
    /**
     * Text to display when `value` is empty.
     */
    readonly placeholder?: string;
    /**
     * Listen to user's input. Useful in case there are multiple input components
     * at the same time and input must be "routed" to a specific component.
     */
    readonly focus?: boolean;
    /**
     * Replace all chars and mask the value. Useful for password inputs.
     */
    readonly mask?: string;
    /**
     * Whether to show cursor and allow navigation inside text input with arrow keys.
     */
    readonly showCursor?: boolean;
    /**
     * Highlight pasted text
     */
    readonly highlightPastedText?: boolean;
    /**
     * Value to display in a text input.
     */
    readonly value: string;
    /**
     * Function to call when value updates.
     */
    readonly onChange: (value: string) => void;
    /**
     * Function to call when `Enter` is pressed, where first argument is a value of the input.
     */
    readonly onSubmit?: (value: string) => void;
};
declare function TextInput({ value: originalValue, placeholder, focus, mask, highlightPastedText, showCursor, onChange, onSubmit, }: Props): React.JSX.Element;
export default TextInput;
type UncontrolledProps = {
    /**
     * Initial value.
     */
    readonly initialValue?: string;
} & Except<Props, 'value' | 'onChange'>;
export declare function UncontrolledTextInput({ initialValue, ...props }: UncontrolledProps): React.JSX.Element;
