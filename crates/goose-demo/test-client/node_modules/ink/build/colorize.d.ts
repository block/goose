type ColorType = 'foreground' | 'background';
declare const colorize: (str: string, color: string | undefined, type: ColorType) => string;
export default colorize;
