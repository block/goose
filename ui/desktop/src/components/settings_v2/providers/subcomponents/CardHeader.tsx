import { memo } from 'react';
import { GreenCheckButton } from './buttons/CardButtons';
import { ConfiguredProviderTooltipMessage, ProviderDescription } from './utils/StringUtils';

interface CardHeaderProps {
  name: string;
  description: string;
  isConfigured: boolean;
}

// Make CardTitle a proper React component
const CardTitle = memo(({ name }: { name: string }) => {
  return <h3 className="text-base font-medium text-textStandard truncate mr-2">{name}</h3>;
});
CardTitle.displayName = 'CardTitle';

// Properly type ProviderNameAndStatus props
interface ProviderNameAndStatusProps {
  name: string;
  isConfigured: boolean;
}

const ProviderNameAndStatus = memo(({ name, isConfigured }: ProviderNameAndStatusProps) => {
  // Remove the console.log completely
  return (
    <div className="flex items-center justify-between w-full">
      <CardTitle name={name} />

      {/* Configured state: Green check */}
      {isConfigured && <GreenCheckButton tooltip={ConfiguredProviderTooltipMessage(name)} />}
    </div>
  );
});
ProviderNameAndStatus.displayName = 'ProviderNameAndStatus';

// Add a container div to the CardHeader
const CardHeader = memo(function CardHeader({ name, description, isConfigured }: CardHeaderProps) {
  return (
    <>
      <ProviderNameAndStatus name={name} isConfigured={isConfigured} />
      <ProviderDescription description={description} />
    </>
  );
});
CardHeader.displayName = 'CardHeader';

export default CardHeader;
