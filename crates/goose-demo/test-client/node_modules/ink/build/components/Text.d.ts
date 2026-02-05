import React, { type ReactNode } from 'react';
import { type ForegroundColorName } from 'chalk';
import { type LiteralUnion } from 'type-fest';
import { type Styles } from '../styles.js';
export type Props = {
    /**
     * Change text color. Ink uses chalk under the hood, so all its functionality is supported.
     */
    readonly color?: LiteralUnion<ForegroundColorName, string>;
    /**
     * Same as `color`, but for background.
     */
    readonly backgroundColor?: LiteralUnion<ForegroundColorName, string>;
    /**
     * Dim the color (emit a small amount of light).
     */
    readonly dimColor?: boolean;
    /**
     * Make the text bold.
     */
    readonly bold?: boolean;
    /**
     * Make the text italic.
     */
    readonly italic?: boolean;
    /**
     * Make the text underlined.
     */
    readonly underline?: boolean;
    /**
     * Make the text crossed with a line.
     */
    readonly strikethrough?: boolean;
    /**
     * Inverse background and foreground colors.
     */
    readonly inverse?: boolean;
    /**
     * This property tells Ink to wrap or truncate text if its width is larger than container.
     * If `wrap` is passed (by default), Ink will wrap text and split it into multiple lines.
     * If `truncate-*` is passed, Ink will truncate text instead, which will result in one line of text with the rest cut off.
     */
    readonly wrap?: Styles['textWrap'];
    readonly children?: ReactNode;
};
/**
 * This component can display text, and change its style to make it colorful, bold, underline, italic or strikethrough.
 */
export default function Text({ color, backgroundColor, dimColor, bold, italic, underline, strikethrough, inverse, wrap, children, }: Props): React.JSX.Element | null;
