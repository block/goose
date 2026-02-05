declare type Callback = (stream: 'stdout' | 'stderr', data: string) => void;
declare type Restore = () => void;
declare const patchConsole: (callback: Callback) => Restore;
export default patchConsole;
