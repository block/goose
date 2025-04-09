import React from 'react';
import { Button } from '../../../../ui/button';
import clsx from 'clsx';
import { TooltipWrapper } from './TooltipWrapper';
import { Check, Rocket, Sliders } from 'lucide-react';

interface ActionButtonProps extends React.ComponentProps<typeof Button> {
  /** Icon component to render, e.g. `RefreshCw` from lucide-react */
  icon?: React.ComponentType<React.SVGProps<globalThis.SVGSVGElement>>;
  /** Tooltip text to show; optional if you want no tooltip. */
  tooltip?: React.ReactNode;
  /** Additional classes for styling. */
  className?: string;
  /** Text to display next to the icon */
  text?: string;
  /** Additional class for the icon specifically */
  iconClassName?: string;
}

// Base styles for all action buttons
const baseActionButtonClasses = `
  rounded-full
  bg-bgApp hover:bg-bgApp shadow-none
  text-textSubtle   
  border border-borderSubtle
  hover:border-borderStandard
  hover:text-textStandard 
  transition-colors
`;

// Additional styles for icon-only buttons
const iconOnlyClasses = `
  h-7 w-7 p-0
`;

// Additional styles for buttons with text and icon
const withTextClasses = `
  px-3 py-1
`;

export function ActionButton({
  icon: Icon,
  size = 'sm',
  variant = 'default',
  tooltip,
  className,
  text,
  iconClassName,
  ...props
}: ActionButtonProps) {
  // Determine if this is an icon-only button or one with text
  const buttonStyle = text
    ? clsx(baseActionButtonClasses, withTextClasses, className)
    : clsx(baseActionButtonClasses, iconOnlyClasses, className);

  const ButtonElement = (
    <Button size={size} variant={variant} className={buttonStyle} {...props}>
      {Icon && <Icon className={clsx('!size-4', iconClassName)} />}
      {text && <span>{text}</span>}
    </Button>
  );

  // If a tooltip is provided, wrap the Button in a tooltip.
  // Otherwise, just return the button as is.
  if (tooltip) {
    return <TooltipWrapper tooltipContent={tooltip}>{ButtonElement}</TooltipWrapper>;
  }

  return ButtonElement;
}

export function GreenCheckButton({
  tooltip,
  className = '', // Provide a default value to prevent undefined errors
  ...props
}: ActionButtonProps) {
  return (
    <ActionButton
      icon={Check}
      tooltip={tooltip}
      className={`
                text-green-600
                dark:text-green-500
                hover:text-green-600
                border-none
                shadow-none
                w-5 h-5
                cursor-default
                ${className}
            `}
      onClick={() => {}}
      {...props}
    />
  );
}

export function ConfigureSettingsButton({ tooltip, className, ...props }: ActionButtonProps) {
  return (
    <ActionButton
      icon={Sliders}
      tooltip={tooltip}
      className={className}
      text={'Configure'}
      iconClassName="rotate-90"
      {...props}
    />
  );
}

export function RocketButton({ tooltip, className, ...props }: ActionButtonProps) {
  return (
    <ActionButton
      icon={Rocket}
      tooltip={tooltip}
      className={className}
      text={'Launch'}
      {...props}
    />
  );
}
