/**
 * Regression test for a dialog opened from a dropdown menu leaving the whole window
 * unclickable after it closes.
 *
 * Radix's `DismissableLayer` sets `pointer-events: none` on `<body>` while a modal layer is
 * open, and restores the value it snapshotted when the last layer of its own copy unmounts.
 * That snapshot is a module-level variable, so it is only correct while one copy of
 * `@radix-ui/react-dismissable-layer` is loaded. Two are: `@radix-ui/react-dialog` resolves
 * 1.1.19 under `ui/desktop`, while the dropdown menu that arrives with `@radix-ui/themes`
 * resolves 1.1.11, hoisted into `ui/node_modules`.
 *
 * A dropdown menu's close animation keeps its layer mounted for a beat after the item that
 * opened the dialog was selected, so the dialog mounts while the menu's `none` is still on
 * the body, snapshots it as the "original", and writes it back on close. Every click in the
 * window is then dead — the sidebar, the session title, the header links.
 *
 * The menu half of that is stood in for here (set the lock, later drop it) rather than
 * rendered: two modal Radix layers mounted at once put jsdom's focus scopes into an
 * infinite focus loop, and the menu's contribution is only ever "set none, restore what was
 * there before". The dialog half is the real component and the real 1.1.19 copy.
 */
import { describe, expect, it, beforeEach, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/react';

import { Dialog, DialogContent, DialogTitle } from './dialog';
import { IntlTestWrapper } from '../../i18n/test-utils';

function JsonDialog({ open }: { open: boolean }) {
  return (
    <IntlTestWrapper>
      <Dialog open={open}>
        <DialogContent>
          <DialogTitle>Session JSON</DialogTitle>
        </DialogContent>
      </Dialog>
    </IntlTestWrapper>
  );
}

async function nextFrames(): Promise<void> {
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
}

describe('a dialog opened while a dropdown menu still holds the body lock', () => {
  beforeEach(() => {
    document.body.style.removeProperty('pointer-events');
  });

  afterEach(() => {
    cleanup();
    document.body.style.removeProperty('pointer-events');
  });

  it('leaves the body clickable after the dialog closes', async () => {
    // The menu that was clicked is still animating out, so its copy of the layer still
    // holds the lock.
    document.body.style.pointerEvents = 'none';

    const { rerender } = render(<JsonDialog open />);

    // The menu finishes closing and its own copy restores what it found: an unlocked body.
    document.body.style.removeProperty('pointer-events');

    rerender(<JsonDialog open={false} />);
    await nextFrames();

    expect(document.body.style.pointerEvents).not.toBe('none');
  });

  it('still locks the body while the dialog is open', () => {
    render(<JsonDialog open />);

    expect(document.body.style.pointerEvents).toBe('none');
  });

  it('leaves the body clickable after a dialog opened on its own closes', async () => {
    const { rerender } = render(<JsonDialog open />);
    expect(document.body.style.pointerEvents).toBe('none');

    rerender(<JsonDialog open={false} />);
    await nextFrames();

    expect(document.body.style.pointerEvents).not.toBe('none');
  });
});
