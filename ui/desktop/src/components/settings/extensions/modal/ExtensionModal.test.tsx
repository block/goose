import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import ExtensionModal from './ExtensionModal';
import { ExtensionFormData } from '../utils';

describe('ExtensionModal', () => {
  it('does not show unsaved changes dialog when closing without modifications', async () => {
    const user = userEvent.setup();
    const mockOnSubmit = vi.fn();
    const mockOnClose = vi.fn();

    const initialData: ExtensionFormData = {
      name: 'Existing Extension',
      description: 'An existing extension',
      type: 'stdio',
      cmd: 'npx some-mcp-server',
      endpoint: '',
      enabled: true,
      timeout: 300,
      envVars: [
        { key: 'API_KEY', value: '••••••••', isEdited: false },
        { key: 'OTHER_VAR', value: '••••••••', isEdited: false },
      ],
      headers: [],
    };

    render(
      <ExtensionModal
        title="Edit Extension"
        initialData={initialData}
        onClose={mockOnClose}
        onSubmit={mockOnSubmit}
        submitLabel="Save"
        modalType="edit"
      />
    );

    const cancelButton = screen.getByRole('button', { name: 'Cancel' });
    await user.click(cancelButton);

    expect(mockOnClose).toHaveBeenCalled();

    expect(screen.queryByText('Unsaved Changes')).not.toBeInTheDocument();
  });

  it('shows unsaved changes dialog when name is modified', async () => {
    const user = userEvent.setup();
    const mockOnSubmit = vi.fn();
    const mockOnClose = vi.fn();

    const initialData: ExtensionFormData = {
      name: 'Original Name',
      description: 'An existing extension',
      type: 'stdio',
      cmd: 'npx some-mcp-server',
      endpoint: '',
      enabled: true,
      timeout: 300,
      envVars: [],
      headers: [],
    };

    render(
      <ExtensionModal
        title="Edit Extension"
        initialData={initialData}
        onClose={mockOnClose}
        onSubmit={mockOnSubmit}
        submitLabel="Save"
        modalType="edit"
      />
    );

    const nameInput = screen.getByPlaceholderText('Enter extension name...');
    await user.clear(nameInput);
    await user.type(nameInput, 'New Name');

    const cancelButton = screen.getByRole('button', { name: 'Cancel' });
    await user.click(cancelButton);

    expect(screen.getByText('Unsaved Changes')).toBeInTheDocument();
    expect(mockOnClose).not.toHaveBeenCalled();
  });

  it('shows unsaved changes dialog when description is modified', async () => {
    const user = userEvent.setup();
    const mockOnSubmit = vi.fn();
    const mockOnClose = vi.fn();

    const initialData: ExtensionFormData = {
      name: 'Test Extension',
      description: 'Original description',
      type: 'stdio',
      cmd: 'npx some-mcp-server',
      endpoint: '',
      enabled: true,
      timeout: 300,
      envVars: [],
      headers: [],
    };

    render(
      <ExtensionModal
        title="Edit Extension"
        initialData={initialData}
        onClose={mockOnClose}
        onSubmit={mockOnSubmit}
        submitLabel="Save"
        modalType="edit"
      />
    );

    const descriptionInput = screen.getByPlaceholderText('Optional description...');
    await user.clear(descriptionInput);
    await user.type(descriptionInput, 'New description');

    const cancelButton = screen.getByRole('button', { name: 'Cancel' });
    await user.click(cancelButton);

    expect(screen.getByText('Unsaved Changes')).toBeInTheDocument();
    expect(mockOnClose).not.toHaveBeenCalled();
  });

  it('shows unsaved changes dialog when timeout is modified', async () => {
    const user = userEvent.setup();
    const mockOnSubmit = vi.fn();
    const mockOnClose = vi.fn();

    const initialData: ExtensionFormData = {
      name: 'Test Extension',
      description: 'An extension',
      type: 'stdio',
      cmd: 'npx some-mcp-server',
      endpoint: '',
      enabled: true,
      timeout: 300,
      envVars: [],
      headers: [],
    };

    render(
      <ExtensionModal
        title="Edit Extension"
        initialData={initialData}
        onClose={mockOnClose}
        onSubmit={mockOnSubmit}
        submitLabel="Save"
        modalType="edit"
      />
    );

    const timeoutInput = screen.getByDisplayValue('300');
    await user.clear(timeoutInput);
    await user.type(timeoutInput, '600');

    const cancelButton = screen.getByRole('button', { name: 'Cancel' });
    await user.click(cancelButton);

    expect(screen.getByText('Unsaved Changes')).toBeInTheDocument();
    expect(mockOnClose).not.toHaveBeenCalled();
  });

  it('creates a http_streamable extension', async () => {
    const user = userEvent.setup();
    const mockOnSubmit = vi.fn();
    const mockOnClose = vi.fn();

    const initialData: ExtensionFormData = {
      name: '',
      description: '',
      type: 'stdio', // Default type
      cmd: '',
      endpoint: '',
      enabled: true,
      timeout: 300,
      envVars: [],
      headers: [],
    };

    render(
      <ExtensionModal
        title="Add custom extension"
        initialData={initialData}
        onClose={mockOnClose}
        onSubmit={mockOnSubmit}
        submitLabel="Add Extension"
        modalType="add"
      />
    );

    const nameInput = screen.getByPlaceholderText('Enter extension name...');
    const submitButton = screen.getByTestId('extension-submit-btn');

    await user.type(nameInput, 'Test MCP');

    const typeSelect = screen.getByRole('combobox');
    await user.click(typeSelect);

    const httpOption = screen.getByText('Streamable HTTP');
    await user.click(httpOption);

    await waitFor(() => {
      expect(screen.getByText('Request Headers')).toBeInTheDocument();
    });

    const endpointInput = screen.getByPlaceholderText('Enter endpoint URL...');
    await user.type(endpointInput, 'https://foo.bar.com/mcp/');

    const descriptionInput = screen.getByPlaceholderText('Optional description...');
    await user.type(descriptionInput, 'Test MCP extension');

    const headerNameInput = screen.getByPlaceholderText('Header name');
    const headerValueInput = screen
      .getAllByPlaceholderText('Value')
      .find(
        (input) =>
          input.closest('div')?.textContent?.includes('Request Headers') ||
          input.parentElement?.parentElement?.textContent?.includes('Request Headers')
      );

    await user.type(headerNameInput, 'Authorization');
    if (headerValueInput) {
      await user.type(headerValueInput, 'Bearer abc123');
    }

    await user.click(submitButton);

    await waitFor(() => {
      expect(mockOnSubmit).toHaveBeenCalled();
    });

    const submittedData = mockOnSubmit.mock.calls[0][0];

    expect(submittedData.name).toBe('Test MCP');
    expect(submittedData.type).toBe('streamable_http');
    expect(submittedData.endpoint).toBe('https://foo.bar.com/mcp/');
    expect(submittedData.description).toBe('Test MCP extension');
    expect(submittedData.timeout).toBe(300);
    expect(submittedData.headers).toHaveLength(1);
    expect(submittedData.headers).toEqual([
      { key: 'Authorization', value: 'Bearer abc123', isEdited: true },
    ]);
  });

  it('masks header values by default (type=password)', async () => {
    const mockOnSubmit = vi.fn();
    const mockOnClose = vi.fn();

    const initialData: ExtensionFormData = {
      name: 'Test Extension',
      description: 'An extension',
      type: 'streamable_http',
      cmd: '',
      endpoint: 'https://example.com/mcp',
      enabled: true,
      timeout: 300,
      envVars: [],
      headers: [{ key: 'Authorization', value: 'Bearer secret123', isEdited: false }],
    };

    render(
      <ExtensionModal
        title="Edit Extension"
        initialData={initialData}
        onClose={mockOnClose}
        onSubmit={mockOnSubmit}
        submitLabel="Save"
        modalType="edit"
      />
    );

    const headerValueInputs = screen.getAllByPlaceholderText('Value');
    expect(headerValueInputs[0]).toHaveAttribute('type', 'password');
  });

  it('reveals header value when toggle is clicked', async () => {
    const user = userEvent.setup();
    const mockOnSubmit = vi.fn();
    const mockOnClose = vi.fn();

    const initialData: ExtensionFormData = {
      name: 'Test Extension',
      description: 'An extension',
      type: 'streamable_http',
      cmd: '',
      endpoint: 'https://example.com/mcp',
      enabled: true,
      timeout: 300,
      envVars: [],
      headers: [{ key: 'Authorization', value: 'Bearer secret123', isEdited: false }],
    };

    render(
      <ExtensionModal
        title="Edit Extension"
        initialData={initialData}
        onClose={mockOnClose}
        onSubmit={mockOnSubmit}
        submitLabel="Save"
        modalType="edit"
      />
    );

    const toggleButtons = screen.getAllByRole('button', { name: 'Show header value' });
    await user.click(toggleButtons[0]);

    const headerValueInputs = screen.getAllByPlaceholderText('Value');
    expect(headerValueInputs[0]).toHaveAttribute('type', 'text');
  });

  it('hides header value when toggle is clicked again', async () => {
    const user = userEvent.setup();
    const mockOnSubmit = vi.fn();
    const mockOnClose = vi.fn();

    const initialData: ExtensionFormData = {
      name: 'Test Extension',
      description: 'An extension',
      type: 'streamable_http',
      cmd: '',
      endpoint: 'https://example.com/mcp',
      enabled: true,
      timeout: 300,
      envVars: [],
      headers: [{ key: 'Authorization', value: 'Bearer secret123', isEdited: false }],
    };

    render(
      <ExtensionModal
        title="Edit Extension"
        initialData={initialData}
        onClose={mockOnClose}
        onSubmit={mockOnSubmit}
        submitLabel="Save"
        modalType="edit"
      />
    );

    const toggleButtons = screen.getAllByRole('button', { name: 'Show header value' });
    await user.click(toggleButtons[0]);

    const hideButtons = screen.getAllByRole('button', { name: 'Hide header value' });
    await user.click(hideButtons[0]);

    const headerValueInputs = screen.getAllByPlaceholderText('Value');
    expect(headerValueInputs[0]).toHaveAttribute('type', 'password');
  });
});
