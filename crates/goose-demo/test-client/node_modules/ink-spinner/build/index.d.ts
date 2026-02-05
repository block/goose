/// <reference types="react" />
import type { SpinnerName } from 'cli-spinners';
type Props = {
    /**
     * Type of a spinner.
     * See [cli-spinners](https://github.com/sindresorhus/cli-spinners) for available spinners.
     *
     * @default dots
     */
    type?: SpinnerName;
};
/**
 * Spinner.
 */
declare function Spinner({ type }: Props): JSX.Element;
export default Spinner;
