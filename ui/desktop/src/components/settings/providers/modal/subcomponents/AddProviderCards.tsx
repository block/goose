import { memo } from 'react';
import CardContainer from '../../subcomponents/CardContainer';
import { Plus } from 'lucide-react';

export const AddProviderCard = memo(function AddProviderCard({ onClick }: { onClick: () => void }) {
  return (
    <CardContainer
      testId="add-provider-card"
      onClick={onClick}
      header={null}
      body={
        <div className="flex flex-col items-center justify-center min-h-[200px]">
          <Plus className="w-8 h-8 text-gray-400 mb-2" />
          <div className="text-sm text-gray-600 dark:text-gray-400 text-center">
            <div className="font-medium">Add Provider</div>
            <div className="text-xs text-gray-500 mt-1">From catalog or manual setup</div>
          </div>
        </div>
      }
      grayedOut={false}
      borderStyle="dashed"
    />
  );
});
