import { AiEnabledEdt } from '@carbon/icons-react';
import type { ComponentPropsWithoutRef } from 'react';

type BrandIconProps = Omit<ComponentPropsWithoutRef<typeof AiEnabledEdt>, 'size'> & {
  size?: number;
};

export function Goose({ className = '', size = 24, ...rest }: BrandIconProps) {
  return (
    <AiEnabledEdt
      size={size}
      className={className}
      data-brand-icon="ibm-carbon-ai-enabled-edt"
      {...rest}
    />
  );
}
