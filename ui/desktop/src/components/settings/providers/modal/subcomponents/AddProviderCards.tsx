import { memo } from 'react';
import CardContainer from '../../subcomponents/CardContainer';
import { Plus, Search } from 'lucide-react';

export const OtherProvidersCard = memo(function OtherProvidersCard({
  onClick,
}: {
  onClick: () => void;
}) {
  return (
    <CardContainer
      testId="add-other-providers-card"
      onClick={onClick}
      header={null}
      body={
        <div className="flex flex-col items-center justify-center min-h-[200px]">
          <Search className="w-8 h-8 text-blue-500 mb-2" />
          <div className="text-sm text-gray-600 dark:text-gray-400 text-center">
            <div className="font-medium">Other Providers</div>
            <div className="text-xs text-gray-500 mt-1">80+ providers</div>
          </div>
        </div>
      }
      grayedOut={false}
      borderStyle="solid"
      className="bg-blue-50 dark:bg-blue-900/10 border-blue-200 dark:border-blue-800 hover:border-blue-400"
    />
  );
});

export const ManualProviderCard = memo(function ManualProviderCard({
  onClick,
}: {
  onClick: () => void;
}) {
  return (
    <CardContainer
      testId="add-manual-provider-card"
      onClick={onClick}
      header={null}
      body={
        <div className="flex flex-col items-center justify-center min-h-[200px]">
          <Plus className="w-8 h-8 text-gray-400 mb-2" />
          <div className="text-sm text-gray-600 dark:text-gray-400 text-center">
            <div>Manual Setup</div>
            <div className="text-xs text-gray-500 mt-1">Custom provider</div>
          </div>
        </div>
      }
      grayedOut={false}
      borderStyle="dashed"
    />
  );
});
