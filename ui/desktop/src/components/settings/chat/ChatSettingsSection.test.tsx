/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import ChatSettingsSection from './ChatSettingsSection';
import { IntlTestWrapper } from '../../../i18n/test-utils';

vi.mock('../mode/ModeSection', () => ({
  ModeSection: () => <div>Mode section</div>,
}));

vi.mock('../dictation/DictationSettings', () => ({
  DictationSettings: () => <div>Dictation settings</div>,
}));

vi.mock('./SpellcheckToggle', () => ({
  SpellcheckToggle: () => <div>Spellcheck toggle</div>,
}));

vi.mock('../response_styles/ResponseStylesSection', () => ({
  ResponseStylesSection: () => <div>Response styles section</div>,
}));

describe('ChatSettingsSection', () => {
  it('hides project hints and prompt injection settings from ApeCloud builds', () => {
    render(<ChatSettingsSection />, { wrapper: IntlTestWrapper });

    expect(screen.getByText('Mode section')).toBeInTheDocument();
    expect(screen.queryByText('Project Hints (.goosehints)')).not.toBeInTheDocument();
    expect(screen.queryByText('Enable Prompt Injection Detection')).not.toBeInTheDocument();
  });
});
