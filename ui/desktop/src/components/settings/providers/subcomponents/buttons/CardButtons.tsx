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

export function ActionButton({
  icon: Icon,
  size = 'sm',
  variant = 'outline',
  tooltip,
  className,
  text,
  iconClassName,
  ...props
}: ActionButtonProps) {
  const ButtonElement = (
    <Button
      size={size}
      variant={variant}
      shape={text ? 'pill' : 'round'}
      className={className}
      {...props}
    >
      {Icon && <Icon className={clsx('!size-4', iconClassName)} />}
      {text && <span>{text}</span>}
    </Button>
  );

  if (tooltip) {
    return <TooltipWrapper tooltipContent={tooltip}>{ButtonElement}</TooltipWrapper>;
  }

  return ButtonElement;
}

export function GreenCheckButton({ tooltip, className = '', ...props }: ActionButtonProps) {
  return (
    <ActionButton
      icon={Check}
      tooltip={tooltip}
      variant="ghost"
      size="sm"
      className={clsx(
        'text-green-600 dark:text-green-500 hover:text-green-600 cursor-default',
        className
      )}
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
      variant="outline"
      text={'Configure'}
      iconClassName="rotate-90"
      className={clsx('cursor-pointer', className)}
      {...props}
    />
  );
}

export function RocketButton({ tooltip, className, ...props }: ActionButtonProps) {
  return (
    <ActionButton
      data-testid="provider-launch-button"
      icon={Rocket}
      tooltip={tooltip}
      variant="outline"
      text={'Launch'}
      className={clsx('cursor-pointer', className)}
      {...props}
    />
  );
}
