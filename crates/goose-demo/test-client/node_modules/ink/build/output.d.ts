import { type OutputTransformer } from './render-node-to-output.js';
/**
 * "Virtual" output class
 *
 * Handles the positioning and saving of the output of each node in the tree.
 * Also responsible for applying transformations to each character of the output.
 *
 * Used to generate the final output of all nodes before writing it to actual output stream (e.g. stdout)
 */
type Options = {
    width: number;
    height: number;
};
type Clip = {
    x1: number | undefined;
    x2: number | undefined;
    y1: number | undefined;
    y2: number | undefined;
};
export default class Output {
    width: number;
    height: number;
    private readonly operations;
    constructor(options: Options);
    write(x: number, y: number, text: string, options: {
        transformers: OutputTransformer[];
    }): void;
    clip(clip: Clip): void;
    unclip(): void;
    get(): {
        output: string;
        height: number;
    };
}
export {};
