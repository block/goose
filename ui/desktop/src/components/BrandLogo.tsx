import { Goose } from './icons/Goose';
import { useWhiteLabel } from '../whitelabel/WhiteLabelContext';

interface BrandLogoProps {
  className?: string;
  size?: 'sm' | 'md' | 'lg';
}

const SIZE_MAP = {
  sm: 'size-5',
  md: 'size-8',
  lg: 'size-10',
};

export default function BrandLogo({ className, size = 'md' }: BrandLogoProps) {
  const { branding } = useWhiteLabel();
  const sizeClass = SIZE_MAP[size];

  if (branding.logo) {
    return <img src={branding.logo} alt={branding.appName} className={className ?? sizeClass} />;
  }

  return <Goose className={className ?? sizeClass} />;
}
