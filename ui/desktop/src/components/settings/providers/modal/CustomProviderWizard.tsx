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

type WizardStep = 'catalog' | 'form';

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
        return selectedTemplate ? `Configure ${selectedTemplate.name}` : 'Configure Provider';
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

        {/* Catalog picker step */}
        {step === 'catalog' && !initialData && (
          <ProviderCatalogPicker onSelect={handleTemplateSelect} onCancel={handleCancel} />
        )}

        {/* Form step */}
        {step === 'form' && (
          <>
            {!initialData && selectedTemplate && (
              <Button type="button" variant="ghost" size="sm" onClick={handleBack} className="mb-2">
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
