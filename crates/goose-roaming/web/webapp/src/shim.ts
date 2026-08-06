// Browser shim for the Electron surface the reused desktop components call.
// MarkdownContent uses window.electron.openExternal (safe-link confirm flow)
// and showMessageBox (error fallback); i18n reads window.appConfig if present.
if (!("electron" in window) || !window.electron) {
  window.electron = {
    async openExternal(url: string) {
      window.open(url, "_blank", "noopener,noreferrer");
    },
    async showMessageBox(opts) {
      window.alert([opts.title, opts.message, opts.detail].filter(Boolean).join("\n"));
      return { response: 0 };
    },
  };
}
