import { type Boxes, type BoxStyle } from 'cli-boxes';
import { type LiteralUnion } from 'type-fest';
import { type ForegroundColorName } from 'ansi-styles';
import { type Node as YogaNode } from 'yoga-layout';
export type Styles = {
    readonly textWrap?: 'wrap' | 'end' | 'middle' | 'truncate-end' | 'truncate' | 'truncate-middle' | 'truncate-start';
    readonly position?: 'absolute' | 'relative';
    /**
     * Size of the gap between an element's columns.
     */
    readonly columnGap?: number;
    /**
     * Size of the gap between element's rows.
     */
    readonly rowGap?: number;
    /**
     * Size of the gap between an element's columns and rows. Shorthand for `columnGap` and `rowGap`.
     */
    readonly gap?: number;
    /**
     * Margin on all sides. Equivalent to setting `marginTop`, `marginBottom`, `marginLeft` and `marginRight`.
     */
    readonly margin?: number;
    /**
     * Horizontal margin. Equivalent to setting `marginLeft` and `marginRight`.
     */
    readonly marginX?: number;
    /**
     * Vertical margin. Equivalent to setting `marginTop` and `marginBottom`.
     */
    readonly marginY?: number;
    /**
     * Top margin.
     */
    readonly marginTop?: number;
    /**
     * Bottom margin.
     */
    readonly marginBottom?: number;
    /**
     * Left margin.
     */
    readonly marginLeft?: number;
    /**
     * Right margin.
     */
    readonly marginRight?: number;
    /**
     * Padding on all sides. Equivalent to setting `paddingTop`, `paddingBottom`, `paddingLeft` and `paddingRight`.
     */
    readonly padding?: number;
    /**
     * Horizontal padding. Equivalent to setting `paddingLeft` and `paddingRight`.
     */
    readonly paddingX?: number;
    /**
     * Vertical padding. Equivalent to setting `paddingTop` and `paddingBottom`.
     */
    readonly paddingY?: number;
    /**
     * Top padding.
     */
    readonly paddingTop?: number;
    /**
     * Bottom padding.
     */
    readonly paddingBottom?: number;
    /**
     * Left padding.
     */
    readonly paddingLeft?: number;
    /**
     * Right padding.
     */
    readonly paddingRight?: number;
    /**
     * This property defines the ability for a flex item to grow if necessary.
     * See [flex-grow](https://css-tricks.com/almanac/properties/f/flex-grow/).
     */
    readonly flexGrow?: number;
    /**
     * It specifies the “flex shrink factor”, which determines how much the flex item will shrink relative to the rest of the flex items in the flex container when there isn’t enough space on the row.
     * See [flex-shrink](https://css-tricks.com/almanac/properties/f/flex-shrink/).
     */
    readonly flexShrink?: number;
    /**
     * It establishes the main-axis, thus defining the direction flex items are placed in the flex container.
     * See [flex-direction](https://css-tricks.com/almanac/properties/f/flex-direction/).
     */
    readonly flexDirection?: 'row' | 'column' | 'row-reverse' | 'column-reverse';
    /**
     * It specifies the initial size of the flex item, before any available space is distributed according to the flex factors.
     * See [flex-basis](https://css-tricks.com/almanac/properties/f/flex-basis/).
     */
    readonly flexBasis?: number | string;
    /**
     * It defines whether the flex items are forced in a single line or can be flowed into multiple lines. If set to multiple lines, it also defines the cross-axis which determines the direction new lines are stacked in.
     * See [flex-wrap](https://css-tricks.com/almanac/properties/f/flex-wrap/).
     */
    readonly flexWrap?: 'nowrap' | 'wrap' | 'wrap-reverse';
    /**
     * The align-items property defines the default behavior for how items are laid out along the cross axis (perpendicular to the main axis).
     * See [align-items](https://css-tricks.com/almanac/properties/a/align-items/).
     */
    readonly alignItems?: 'flex-start' | 'center' | 'flex-end' | 'stretch';
    /**
     * It makes possible to override the align-items value for specific flex items.
     * See [align-self](https://css-tricks.com/almanac/properties/a/align-self/).
     */
    readonly alignSelf?: 'flex-start' | 'center' | 'flex-end' | 'auto';
    /**
     * It defines the alignment along the main axis.
     * See [justify-content](https://css-tricks.com/almanac/properties/j/justify-content/).
     */
    readonly justifyContent?: 'flex-start' | 'flex-end' | 'space-between' | 'space-around' | 'space-evenly' | 'center';
    /**
     * Width of the element in spaces.
     * You can also set it in percent, which will calculate the width based on the width of parent element.
     */
    readonly width?: number | string;
    /**
     * Height of the element in lines (rows).
     * You can also set it in percent, which will calculate the height based on the height of parent element.
     */
    readonly height?: number | string;
    /**
     * Sets a minimum width of the element.
     */
    readonly minWidth?: number | string;
    /**
     * Sets a minimum height of the element.
     */
    readonly minHeight?: number | string;
    /**
     * Set this property to `none` to hide the element.
     */
    readonly display?: 'flex' | 'none';
    /**
     * Add a border with a specified style.
     * If `borderStyle` is `undefined` (which it is by default), no border will be added.
     */
    readonly borderStyle?: keyof Boxes | BoxStyle;
    /**
     * Determines whether top border is visible.
     *
     * @default true
     */
    readonly borderTop?: boolean;
    /**
     * Determines whether bottom border is visible.
     *
     * @default true
     */
    readonly borderBottom?: boolean;
    /**
     * Determines whether left border is visible.
     *
     * @default true
     */
    readonly borderLeft?: boolean;
    /**
     * Determines whether right border is visible.
     *
     * @default true
     */
    readonly borderRight?: boolean;
    /**
     * Change border color.
     * Shorthand for setting `borderTopColor`, `borderRightColor`, `borderBottomColor` and `borderLeftColor`.
     */
    readonly borderColor?: LiteralUnion<ForegroundColorName, string>;
    /**
     * Change top border color.
     * Accepts the same values as `color` in `Text` component.
     */
    readonly borderTopColor?: LiteralUnion<ForegroundColorName, string>;
    /**
     * Change bottom border color.
     * Accepts the same values as `color` in `Text` component.
     */
    readonly borderBottomColor?: LiteralUnion<ForegroundColorName, string>;
    /**
     * Change left border color.
     * Accepts the same values as `color` in `Text` component.
     */
    readonly borderLeftColor?: LiteralUnion<ForegroundColorName, string>;
    /**
     * Change right border color.
     * Accepts the same values as `color` in `Text` component.
     */
    readonly borderRightColor?: LiteralUnion<ForegroundColorName, string>;
    /**
     * Dim the border color.
     * Shorthand for setting `borderTopDimColor`, `borderBottomDimColor`, `borderLeftDimColor` and `borderRightDimColor`.
     *
     * @default false
     */
    readonly borderDimColor?: boolean;
    /**
     * Dim the top border color.
     *
     * @default false
     */
    readonly borderTopDimColor?: boolean;
    /**
     * Dim the bottom border color.
     *
     * @default false
     */
    readonly borderBottomDimColor?: boolean;
    /**
     * Dim the left border color.
     *
     * @default false
     */
    readonly borderLeftDimColor?: boolean;
    /**
     * Dim the right border color.
     *
     * @default false
     */
    readonly borderRightDimColor?: boolean;
    /**
     * Behavior for an element's overflow in both directions.
     *
     * @default 'visible'
     */
    readonly overflow?: 'visible' | 'hidden';
    /**
     * Behavior for an element's overflow in horizontal direction.
     *
     * @default 'visible'
     */
    readonly overflowX?: 'visible' | 'hidden';
    /**
     * Behavior for an element's overflow in vertical direction.
     *
     * @default 'visible'
     */
    readonly overflowY?: 'visible' | 'hidden';
};
declare const styles: (node: YogaNode, style?: Styles) => void;
export default styles;
