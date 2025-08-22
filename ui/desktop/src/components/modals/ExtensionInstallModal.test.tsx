import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ExtensionInstallModal } from './ExtensionInstallModal';
import { ModalType, ExtensionModalConfig } from '../../types/extension';

// Mock dependencies
vi.mock('../ui/dialog', () => ({
  Dialog: ({
    children,
    open,
    onOpenChange,
  }: {
    children: React.ReactNode;
    open: boolean;
    onOpenChange: (open: boolean) => void;
  }) =>
    open ? (
      <div data-testid="dialog" onClick={() => onOpenChange(false)}>
        {children}
      </div>
    ) : null,
  DialogContent: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <div data-testid="dialog-content" className={className}>
      {children}
    </div>
  ),
  DialogHeader: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="dialog-header">{children}</div>
  ),
  DialogTitle: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <h1 data-testid="dialog-title" className={className}>
      {children}
    </h1>
  ),
  DialogDescription: ({
    children,
    className,
  }: {
    children: React.ReactNode;
    className?: string;
  }) => (
    <div data-testid="dialog-description" className={className}>
      {children}
    </div>
  ),
  DialogFooter: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <div data-testid="dialog-footer" className={className}>
      {children}
    </div>
  ),
}));

vi.mock('../ui/button', () => ({
  Button: ({
    children,
    onClick,
    disabled,
    variant,
    ...props
  }: {
    children: React.ReactNode;
    onClick?: () => void;
    disabled?: boolean;
    variant?: string;
    [key: string]: unknown;
  }) => (
    <button
      onClick={onClick}
      disabled={disabled}
      data-variant={variant}
      data-testid={props['data-testid'] || 'button'}
      {...props}
    >
      {children}
    </button>
  ),
}));

describe('ExtensionInstallModal', () => {
  const mockOnConfirm = vi.fn();
  const mockOnCancel = vi.fn();

  const createConfig = (type: ModalType): ExtensionModalConfig => {
    switch (type) {
      case 'blocked':
        return {
          title: 'Extension Installation Blocked',
          message:
            "This extension cannot be installed because it is not on your organization's approved list.\n\nExtension: TestExt\nCommand: npx test-ext\n\nContact your administrator to request approval for this extension.",
          confirmLabel: 'OK',
          cancelLabel: '',
          showSingleButton: true,
          isBlocked: true,
        };

      case 'untrusted':
        return {
          title: 'Install Untrusted Extension?',
          message:
            "This extension is not on your organization's approved list and may pose security risks.\n\nExtension: TestExt\nCommand: npx test-ext\n\nThis extension will be able to access your conversations and provide additional functionality.\n\nOnly install if you trust the source. Contact your administrator if unsure.",
          confirmLabel: 'Install Anyway',
          cancelLabel: 'Cancel',
          showSingleButton: false,
          isBlocked: false,
        };

      case 'trusted':
      default:
        return {
          title: 'Confirm Extension Installation',
          message:
            'Are you sure you want to install the TestExt extension?\n\nCommand: npx test-ext',
          confirmLabel: 'Yes',
          cancelLabel: 'No',
          showSingleButton: false,
          isBlocked: false,
        };
    }
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Modal Rendering', () => {
    it('should not render when config is null', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="trusted"
          config={null}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      expect(screen.queryByTestId('dialog')).not.toBeInTheDocument();
    });

    it('should not render when isOpen is false', () => {
      render(
        <ExtensionInstallModal
          isOpen={false}
          modalType="trusted"
          config={createConfig('trusted')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      expect(screen.queryByTestId('dialog')).not.toBeInTheDocument();
    });

    it('should render when isOpen is true and config exists', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="trusted"
          config={createConfig('trusted')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      expect(screen.getByTestId('dialog')).toBeInTheDocument();
      expect(screen.getByTestId('dialog-title')).toHaveTextContent(
        'Confirm Extension Installation'
      );
    });
  });

  describe('Trusted Extension Modal', () => {
    it('should render trusted extension modal correctly', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="trusted"
          config={createConfig('trusted')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      expect(screen.getByTestId('dialog-title')).toHaveTextContent(
        'Confirm Extension Installation'
      );
      expect(screen.getByTestId('dialog-description')).toHaveTextContent(
        'Are you sure you want to install the TestExt extension?'
      );

      // Should have two buttons
      const buttons = screen.getAllByTestId('button');
      expect(buttons).toHaveLength(2);
      expect(buttons[0]).toHaveTextContent('No');
      expect(buttons[1]).toHaveTextContent('Yes');
    });

    it('should call onCancel when cancel button is clicked', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="trusted"
          config={createConfig('trusted')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      const cancelButton = screen.getByText('No');
      fireEvent.click(cancelButton);

      expect(mockOnCancel).toHaveBeenCalled();
      expect(mockOnConfirm).not.toHaveBeenCalled();
    });

    it('should call onConfirm when confirm button is clicked', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="trusted"
          config={createConfig('trusted')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      const confirmButton = screen.getByText('Yes');
      fireEvent.click(confirmButton);

      expect(mockOnConfirm).toHaveBeenCalled();
      // Note: onCancel will also be called once due to Dialog's onOpenChange behavior
    });
  });

  describe('Untrusted Extension Modal', () => {
    it('should render untrusted extension modal correctly', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="untrusted"
          config={createConfig('untrusted')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      expect(screen.getByTestId('dialog-title')).toHaveTextContent('Install Untrusted Extension?');
      expect(screen.getByTestId('dialog-description')).toHaveTextContent(
        "This extension is not on your organization's approved list"
      );

      // Should have two buttons
      const buttons = screen.getAllByTestId('button');
      expect(buttons).toHaveLength(2);
      expect(buttons[0]).toHaveTextContent('Cancel');
      expect(buttons[1]).toHaveTextContent('Install Anyway');
    });

    it('should apply warning styling for untrusted modal', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="untrusted"
          config={createConfig('untrusted')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      const title = screen.getByTestId('dialog-title');
      expect(title).toHaveClass('text-yellow-600', 'dark:text-yellow-400');
    });

    it('should use destructive variant for confirm button', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="untrusted"
          config={createConfig('untrusted')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      const confirmButton = screen.getByText('Install Anyway');
      expect(confirmButton).toHaveAttribute('data-variant', 'destructive');
    });
  });

  describe('Blocked Extension Modal', () => {
    it('should render blocked extension modal correctly', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="blocked"
          config={createConfig('blocked')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      expect(screen.getByTestId('dialog-title')).toHaveTextContent(
        'Extension Installation Blocked'
      );
      expect(screen.getByTestId('dialog-description')).toHaveTextContent(
        'This extension cannot be installed'
      );

      // Should have only one button
      const buttons = screen.getAllByTestId('button');
      expect(buttons).toHaveLength(1);
      expect(buttons[0]).toHaveTextContent('OK');
    });

    it('should apply error styling for blocked modal', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="blocked"
          config={createConfig('blocked')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      const title = screen.getByTestId('dialog-title');
      expect(title).toHaveClass('text-red-600', 'dark:text-red-400');
    });

    it('should call onCancel when OK button is clicked (single button)', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="blocked"
          config={createConfig('blocked')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      const okButton = screen.getByText('OK');
      fireEvent.click(okButton);

      expect(mockOnCancel).toHaveBeenCalled();
      expect(mockOnConfirm).not.toHaveBeenCalled();
    });
  });

  describe('Loading States', () => {
    it('should disable buttons when isSubmitting is true', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="trusted"
          config={createConfig('trusted')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
          isSubmitting={true}
        />
      );

      const buttons = screen.getAllByTestId('button');
      buttons.forEach((button) => {
        expect(button).toBeDisabled();
      });
    });

    it('should show "Installing..." text when isSubmitting is true', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="trusted"
          config={createConfig('trusted')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
          isSubmitting={true}
        />
      );

      expect(screen.getByText('Installing...')).toBeInTheDocument();
    });
  });

  describe('Message Formatting', () => {
    it('should preserve line breaks in modal message', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="trusted"
          config={createConfig('trusted')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      const description = screen.getByTestId('dialog-description');
      expect(description).toHaveClass('whitespace-pre-wrap');
    });

    it('should align text to the left', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="trusted"
          config={createConfig('trusted')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      const description = screen.getByTestId('dialog-description');
      expect(description).toHaveClass('text-left');
    });
  });

  describe('Dialog Interaction', () => {
    it('should call onCancel when dialog is closed via onOpenChange', () => {
      render(
        <ExtensionInstallModal
          isOpen={true}
          modalType="trusted"
          config={createConfig('trusted')}
          onConfirm={mockOnConfirm}
          onCancel={mockOnCancel}
        />
      );

      // Click outside dialog (simulated by our mock)
      const dialog = screen.getByTestId('dialog');
      fireEvent.click(dialog);

      expect(mockOnCancel).toHaveBeenCalled();
    });
  });
});
