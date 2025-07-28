import { useState } from 'react';
import { Button } from '../../../ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../../../ui/dialog';
import { ExtensionFormData } from '../utils';
import EnvVarsSection from './EnvVarsSection';
import HeadersSection from './HeadersSection';
import ExtensionConfigFields from './ExtensionConfigFields';
import { PlusIcon, Edit, Trash2, AlertTriangle } from 'lucide-react';
import ExtensionInfoFields from './ExtensionInfoFields';
import ExtensionTimeoutField from './ExtensionTimeoutField';
import { upsertConfig } from '../../../../api/sdk.gen';
import { ConfirmationModal } from '../../../ui/ConfirmationModal';

interface ExtensionModalProps {
  title: string;
  initialData: ExtensionFormData;
  onClose: () => void;
  onSubmit: (formData: ExtensionFormData) => void;
  onDelete?: (name: string) => void;
  submitLabel: string;
  modalType: 'add' | 'edit';
}

export default function ExtensionModal({
  title,
  initialData,
  onClose,
  onSubmit,
  onDelete,
  submitLabel,
  modalType,
}: ExtensionModalProps) {
  const [formData, setFormData] = useState<ExtensionFormData>(initialData);
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);
  const [submitAttempted, setSubmitAttempted] = useState(false);
  const [showCloseConfirmation, setShowCloseConfirmation] = useState(false);
  const [hasPendingEnvVars, setHasPendingEnvVars] = useState(false);
  const [hasPendingHeaders, setHasPendingHeaders] = useState(false);

  // Function to check if form has been modified
  const hasFormChanges = (): boolean => {
    // Check if command/endpoint has changed
    const commandChanged =
      (formData.type === 'stdio' && formData.cmd !== initialData.cmd) ||
      (formData.type === 'sse' && formData.endpoint !== initialData.endpoint) ||
      (formData.type === 'streamable_http' && formData.endpoint !== initialData.endpoint);

    // Check if headers have changed
    const headersChanged = formData.headers.some((header) => header.isEdited === true);

    // Check if any environment variables have been modified
    const envVarsChanged = formData.envVars.some((envVar) => envVar.isEdited === true);

    // Check if new env vars have been added
    const envVarsAdded = formData.envVars.length > initialData.envVars.length;

    // Check if env vars have been removed
    const envVarsRemoved = formData.envVars.length < initialData.envVars.length;

    // Check if any environment variable fields have text entered (even if not marked as edited)
    const envVarsHaveText = formData.envVars.some(
      (envVar) =>
        (envVar.key.trim() !== '' || envVar.value.trim() !== '') &&
        // Don't count placeholder values for existing secrets
        envVar.value !== '••••••••'
    );

    // Check if there are pending environment variables being typed
    const hasPendingInput = hasPendingEnvVars || hasPendingHeaders;

    return (
      commandChanged ||
      headersChanged ||
      envVarsChanged ||
      envVarsAdded ||
      envVarsRemoved ||
      envVarsHaveText ||
      hasPendingInput
    );
  };

  // Handle backdrop close with confirmation if needed
  const handleClose = () => {
    if (hasFormChanges()) {
      setShowCloseConfirmation(true);
    } else {
      onClose();
    }
  };

  // Handle confirmed close
  const handleConfirmClose = () => {
    setShowCloseConfirmation(false);
    onClose();
  };

  // Handle cancel close confirmation
  const handleCancelClose = () => {
    setShowCloseConfirmation(false);
  };

  const handleAddEnvVar = (key: string, value: string) => {
    setFormData({
      ...formData,
      envVars: [...formData.envVars, { key, value, isEdited: true }],
    });
  };

  const handleRemoveEnvVar = (index: number) => {
    const newEnvVars = [...formData.envVars];
    newEnvVars.splice(index, 1);
    setFormData({
      ...formData,
      envVars: newEnvVars,
    });
  };

  const handleEnvVarChange = (index: number, field: 'key' | 'value', value: string) => {
    const newEnvVars = [...formData.envVars];
    newEnvVars[index][field] = value;

    // Mark as edited if it's a value change
    if (field === 'value') {
      newEnvVars[index].isEdited = true;
    }

    setFormData({
      ...formData,
      envVars: newEnvVars,
    });
  };

  const handleAddHeader = (key: string, value: string) => {
    setFormData({
      ...formData,
      headers: [...formData.headers, { key, value, isEdited: true }],
    });
  };

  const handleRemoveHeader = (index: number) => {
    const newHeaders = [...formData.headers];
    newHeaders.splice(index, 1);
    setFormData({
      ...formData,
      headers: newHeaders,
    });
  };

  const handleHeaderChange = (index: number, field: 'key' | 'value', value: string) => {
    const newHeaders = [...formData.headers];
    newHeaders[index][field] = value;

    // Mark as edited if it's a value change
    if (field === 'value') {
      newHeaders[index].isEdited = true;
    }

    setFormData({
      ...formData,
      headers: newHeaders,
    });
  };

  // Function to store a secret value
  const storeSecret = async (key: string, value: string) => {
    try {
      await upsertConfig({
        body: {
          is_secret: true,
          key: key,
          value: value,
        },
      });
      return true;
    } catch (error) {
      console.error('Failed to store secret:', error);
      return false;
    }
  };

  // Function to determine which icon to display with proper styling
  const getModalIcon = () => {
    if (showDeleteConfirmation) {
      return <AlertTriangle className="text-red-500" size={24} />;
    }
    return modalType === 'add' ? (
      <PlusIcon className="text-iconStandard" size={24} />
    ) : (
      <Edit className="text-iconStandard" size={24} />
    );
  };

  const isNameValid = () => {
    return formData.name.trim() !== '';
  };

  const isConfigValid = () => {
    return (
      (formData.type === 'stdio' && !!formData.cmd && formData.cmd.trim() !== '') ||
      (formData.type === 'sse' && !!formData.endpoint && formData.endpoint.trim() !== '') ||
      (formData.type === 'streamable_http' &&
        !!formData.endpoint &&
        formData.endpoint.trim() !== '')
    );
  };

  const isEnvVarsValid = () => {
    return formData.envVars.every(
      ({ key, value }) => (key === '' && value === '') || (key !== '' && value !== '')
    );
  };

  const isHeadersValid = () => {
    return formData.headers.every(
      ({ key, value }) => (key === '' && value === '') || (key !== '' && value !== '')
    );
  };

  const isTimeoutValid = () => {
    // Check if timeout is not undefined, null, or empty string
    if (formData.timeout === undefined || formData.timeout === null) {
      return false;
    }

    // Convert to number if it's a string
    const timeoutValue =
      typeof formData.timeout === 'string' ? Number(formData.timeout) : formData.timeout;

    // Check if it's a valid number (not NaN) and is a positive number
    return !isNaN(timeoutValue) && timeoutValue > 0;
  };

  // Form validation
  const isFormValid = () => {
    return (
      isNameValid() && isConfigValid() && isEnvVarsValid() && isHeadersValid() && isTimeoutValid()
    );
  };

  // Handle submit with validation and secret storage
  const handleSubmit = async () => {
    setSubmitAttempted(true);

    if (isFormValid()) {
      // Only store env vars that have been edited (which includes new)
      const secretPromises = formData.envVars
        .filter((envVar) => envVar.isEdited)
        .map(({ key, value }) => storeSecret(key, value));

      try {
        // Wait for all secrets to be stored
        const results = await Promise.all(secretPromises);

        if (results.every((success) => success)) {
          // Convert timeout to number if needed
          const dataToSubmit = {
            ...formData,
            timeout:
              typeof formData.timeout === 'string' ? Number(formData.timeout) : formData.timeout,
          };
          onSubmit(dataToSubmit);
          onClose();
        } else {
          console.error('Failed to store one or more secrets');
        }
      } catch (error) {
        console.error('Error during submission:', error);
      }
    } else {
      console.log('Form validation failed');
    }
  };

  // Update title based on current state
  const modalTitle = showDeleteConfirmation ? `Delete Extension "${formData.name}"` : title;

  return (
    <>
      <Dialog open={true} onOpenChange={handleClose}>
        <DialogContent className="sm:max-w-[600px] max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              {getModalIcon()}
              {modalTitle}
            </DialogTitle>
            {showDeleteConfirmation && (
              <DialogDescription>
                This will permanently remove this extension and all of its settings.
              </DialogDescription>
            )}
          </DialogHeader>

          {showDeleteConfirmation ? (
            <div className="py-4">
              <p className="text-textStandard">
                This will permanently remove this extension and all of its settings.
              </p>
            </div>
          ) : (
            <div className="py-4 space-y-6">
              {/* Form Fields */}
              {/* Name and Type */}
              <ExtensionInfoFields
                name={formData.name}
                type={formData.type}
                description={formData.description}
                onChange={(key, value) => setFormData({ ...formData, [key]: value })}
                submitAttempted={submitAttempted}
              />

              {/* Divider */}
              <hr className="border-t border-borderSubtle" />

              {/* Command */}
              <div>
                <ExtensionConfigFields
                  type={formData.type}
                  full_cmd={formData.cmd || ''}
                  endpoint={formData.endpoint || ''}
                  onChange={(key, value) => setFormData({ ...formData, [key]: value })}
                  submitAttempted={submitAttempted}
                  isValid={isConfigValid()}
                />
                <div className="mb-4" />
                <ExtensionTimeoutField
                  timeout={formData.timeout || 300}
                  onChange={(key, value) => setFormData({ ...formData, [key]: value })}
                  submitAttempted={submitAttempted}
                />
              </div>

              {/* Divider */}
              <hr className="border-t border-borderSubtle" />

              {/* Environment Variables */}
              <div>
                <EnvVarsSection
                  envVars={formData.envVars}
                  onAdd={handleAddEnvVar}
                  onRemove={handleRemoveEnvVar}
                  onChange={handleEnvVarChange}
                  submitAttempted={submitAttempted}
                  onPendingInputChange={setHasPendingEnvVars}
                />
              </div>
            </div>
          )}

          {/* Request Headers - Only for streamable_http */}
          {formData.type === 'streamable_http' && (
            <>
              {/* Divider */}
              <hr className="border-t border-borderSubtle mb-4" />

              <div className="mb-6">
                <HeadersSection
                  headers={formData.headers}
                  onAdd={handleAddHeader}
                  onRemove={handleRemoveHeader}
                  onChange={handleHeaderChange}
                  submitAttempted={submitAttempted}
                  onPendingInputChange={setHasPendingHeaders}
                />
              </div>
            </>
          )}

          <DialogFooter className="pt-2">
            {showDeleteConfirmation ? (
              <>
                <Button variant="outline" onClick={() => setShowDeleteConfirmation(false)}>
                  Cancel
                </Button>
                <Button
                  onClick={() => {
                    if (onDelete) {
                      onDelete(formData.name);
                      onClose();
                    }
                  }}
                  variant="destructive"
                >
                  <Trash2 className="h-4 w-4 mr-2" />
                  Confirm removal
                </Button>
              </>
            ) : (
              <>
                {modalType === 'edit' && onDelete && (
                  <Button
                    onClick={() => setShowDeleteConfirmation(true)}
                    variant="outline"
                    className="text-red-500 hover:text-red-600"
                  >
                    <Trash2 className="h-4 w-4 mr-2" />
                    Remove extension
                  </Button>
                )}
                <Button variant="outline" onClick={handleClose}>
                  Cancel
                </Button>
                <Button onClick={handleSubmit} disabled={!isFormValid()}>
                  {submitLabel}
                </Button>
              </>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Close Confirmation Modal */}
      {showCloseConfirmation && (
        <ConfirmationModal
          isOpen={showCloseConfirmation}
          title="Unsaved Changes"
          message="You have unsaved changes to the extension configuration. Are you sure you want to close without saving?"
          confirmLabel="Close Without Saving"
          onConfirm={handleConfirmClose}
          onCancel={handleCancelClose}
        />
      )}
    </>
  );
}
