import { escape } from './escape.mjs';

declare const templateSettings: {
    escape: RegExp;
    evaluate: RegExp;
    interpolate: RegExp;
    variable: string;
    imports: {
        _: {
            escape: typeof escape;
            template: typeof template;
        };
    };
};
interface TemplateOptions {
    escape?: RegExp | null | undefined;
    evaluate?: RegExp | null | undefined;
    interpolate?: RegExp | null | undefined;
    variable?: string | undefined;
    imports?: Record<string, any> | undefined;
    sourceURL?: string;
}
interface TemplateExecutor {
    (data?: object): string;
    source: string;
}
declare function template(string?: string, options?: TemplateOptions): TemplateExecutor;

export { template, templateSettings };
