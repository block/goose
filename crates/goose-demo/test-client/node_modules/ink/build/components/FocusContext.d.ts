export type Props = {
    readonly activeId?: string;
    readonly add: (id: string, options: {
        autoFocus: boolean;
    }) => void;
    readonly remove: (id: string) => void;
    readonly activate: (id: string) => void;
    readonly deactivate: (id: string) => void;
    readonly enableFocus: () => void;
    readonly disableFocus: () => void;
    readonly focusNext: () => void;
    readonly focusPrevious: () => void;
    readonly focus: (id: string) => void;
};
declare const FocusContext: import("react").Context<Props>;
export default FocusContext;
