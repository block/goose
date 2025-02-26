import React from 'react';
import { Button } from '../../../../../ui/button';
import clsx from 'clsx';
import { TooltipWrapper } from './TooltipWrapper';
import { Check, CircleHelp, Plus, RefreshCw, Rocket, Settings, X } from 'lucide-react';

interface ActionButtonProps extends React.ComponentProps<typeof Button> {
  /** Icon component to render, e.g. `RefreshCw` from lucide-react */
  icon?: React.ComponentType<React.SVGProps<globalThis.SVGSVGElement>>;
  /** Tooltip text to show; optional if you want no tooltip. */
  tooltip?: React.ReactNode;
  /** Additional classes for styling. */
  className?: string;
}

// className is the styling for the <Button/> component -- below is the default
const baseActionButtonClasses = `
  rounded-full h-7 w-7 p-0
  bg-bgApp hover:bg-bgApp shadow-none
  text-textSubtle   
  border border-borderSubtle
  hover:border-borderStandard
  hover:text-textStandard 
  transition-colors
`;

export function ActionButton({
  icon: Icon,
  size = 'sm',
  variant = 'default',
  tooltip,
  className,
  ...props
}: ActionButtonProps) {
  const ButtonElement = (
    <Button
      size={size}
      variant={variant}
      className={clsx(baseActionButtonClasses, className)}
      {...props}
    >
      {Icon && <Icon className="!size-4" />}
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
                bg-green-100
                dark:bg-green-900/30
                text-green-600
                dark:text-green-500
                hover:bg-green-100
                hover:text-green-600
                border-none
                shadow-none
                w-5 h-5
                cursor-default
                ${className}  // Removed the nullish coalescing operator as default is provided
            `}
      onClick={() => {}}
      {...props}
    />
  );
}

export function ExclamationButton({ tooltip, className, ...props }: ActionButtonProps) {
  return <ActionButton icon={CircleHelp} tooltip={tooltip} onClick={() => {}} {...props} />;
}

export function GearSettingsButton({ tooltip, className, ...props }: ActionButtonProps) {
  return <ActionButton icon={Settings} tooltip={tooltip} className={className} {...props} />;
}

export function AddButton({ tooltip, className, ...props }: ActionButtonProps) {
  return <ActionButton icon={Plus} tooltip={tooltip} className={className} {...props} />;
}

export function DeleteButton({ tooltip, className, ...props }: ActionButtonProps) {
  return <ActionButton icon={X} tooltip={tooltip} className={className} {...props} />;
}

export function RefreshButton({ tooltip, className, ...props }: ActionButtonProps) {
  return <ActionButton icon={RefreshCw} tooltip={tooltip} className={className} {...props} />;
}

export function RocketButton({ tooltip, className, ...props }: ActionButtonProps) {
  return <ActionButton icon={Rocket} tooltip={tooltip} className={className} {...props} />;
}
