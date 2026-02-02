import React, { useState } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../../../ui/dialog';
import { Button } from '../../../ui/button';
import { UpdateCustomProviderRequest } from '../../../../api';
import ProviderCatalogPicker from './subcomponents/ProviderCatalogPicker';
import EnhancedCustomProviderForm from './subcomponents/forms/EnhancedCustomProviderForm';

interface ProviderTemplate {
  id: string;
  name: string;
  format: string;
  api_url: string;
  models: Array<{
    id: string;
    name: string;
    context_limit: number;
    capabilities: {
      tool_call: boolean;
      reasoning: boolean;
      attachment: boolean;
      temperature: boolean;
    };
    deprecated: boolean;
  }>;
  supports_streaming: boolean;
  env_var: string;
  doc_url: string;
}

interface CustomProviderWizardProps {
  open: boolean;
  onClose: () => void;
  onSubmit: (data: UpdateCustomProviderRequest) => void;
  initialData?: UpdateCustomProviderRequest | null;
  isEditable?: boolean;
}

type WizardStep = 'catalog' | 'manual' | 'form';

export default function CustomProviderWizard({
  open,
  onClose,
  onSubmit,
  initialData,
  isEditable = true,
}: CustomProviderWizardProps) {
  const [step, setStep] = useState<WizardStep>('catalog');
  const [selectedTemplate, setSelectedTemplate] = useState<ProviderTemplate | null>(null);

  // Reset state when modal opens/closes
  React.useEffect(() => {
    if (open && !initialData) {
      setStep('catalog');
      setSelectedTemplate(null);
    } else if (open && initialData) {
      setStep('form');
      setSelectedTemplate(null);
    }
  }, [open, initialData]);

  const handleTemplateSelect = (template: ProviderTemplate) => {
    setSelectedTemplate(template);
    setStep('form');
  };

  const handleManualSetup = () => {
    setSelectedTemplate(null);
    setStep('form');
  };

  const handleBack = () => {
    if (step === 'form' && !initialData) {
      setStep('catalog');
      setSelectedTemplate(null);
    }
  };

  const handleCancel = () => {
    setStep('catalog');
    setSelectedTemplate(null);
    onClose();
  };

  const getTitle = () => {
    if (initialData) {
      return 'Edit Custom Provider';
    }
    switch (step) {
      case 'catalog':
        return 'Add Custom Provider';
      case 'form':
        return selectedTemplate ? `Configure ${selectedTemplate.name}` : 'Manual Provider Setup';
      default:
        return 'Add Custom Provider';
    }
  };

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{getTitle()}</DialogTitle>
        </DialogHeader>

        {/* Initial step: Choose catalog or manual */}
        {step === 'catalog' && !initialData && (
          <div className="space-y-4">
            <div>
              <p className="text-sm text-textSubtle mb-4">
                Choose how you'd like to add your provider
              </p>
            </div>

            <div className="space-y-3">
              <button
                onClick={() => setStep('catalog')}
                className="w-full p-4 text-left border border-border rounded-lg hover:bg-surfaceHover transition-colors"
              >
                <div className="flex items-center justify-between">
                  <div>
                    <div className="font-medium text-textStandard">
                      Choose from Catalog
                    </div>
                    <div className="text-sm text-textSubtle mt-1">
                      Select from 80+ providers with auto-filled configuration
                    </div>
                  </div>
                  <div className="text-xs text-textSubtle bg-surfaceHover px-2 py-1 rounded">
                    Recommended
                  </div>
                </div>
              </button>

              <button
                onClick={handleManualSetup}
                className="w-full p-4 text-left border border-border rounded-lg hover:bg-surfaceHover transition-colors"
              >
                <div className="flex items-center justify-between">
                  <div>
                    <div className="font-medium text-textStandard">
                      Manual Setup
                    </div>
                    <div className="text-sm text-textSubtle mt-1">
                      Enter all configuration details manually
                    </div>
                  </div>
                </div>
              </button>
            </div>

            <div className="flex justify-end space-x-2 pt-4 border-t border-border">
              <Button type="button" variant="outline" onClick={handleCancel}>
                Cancel
              </Button>
            </div>
          </div>
        )}

        {/* Catalog picker step - nested component handles format → provider flow */}
        {step === 'catalog' && !initialData && (
          <ProviderCatalogPicker onSelect={handleTemplateSelect} onCancel={handleCancel} />
        )}

        {/* Form step */}
        {step === 'form' && (
          <>
            {!initialData && selectedTemplate && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={handleBack}
                className="mb-2"
              >
                ← Back to catalog
              </Button>
            )}
            <EnhancedCustomProviderForm
              onSubmit={onSubmit}
              onCancel={handleCancel}
              template={selectedTemplate}
              initialData={initialData}
              isEditable={isEditable}
            />
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
