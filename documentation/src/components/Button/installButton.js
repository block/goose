import React from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';

const InstallButton = ({ 
  size = null, 
  outline = false, 
  variant = 'primary', 
  block = false, 
  disabled = false, 
  className, 
  style, 
  link, 
  label 
}) => {
  const sizeMap = {
    sm: 'sm',
    small: 'sm',
    lg: 'lg',
    large: 'lg',
    medium: null,
  };

  const buttonSize = size ? sizeMap[size] : '';
  const sizeClass = buttonSize ? `button--${buttonSize}` : '';
  const outlineClass = outline ? 'button--outline' : '';
  const variantClass = variant ? `button--${variant}` : '';
  const blockClass = block ? 'button--block' : '';
  const disabledClass = disabled ? 'disabled' : '';
  const destination = disabled ? null : link;

  // Replace shortcodes like ":arrow_down:" with actual emojis
  const parsedLabel = label.replace(':arrow_down:', '⬇️');

  return (
    <Link to={destination}>
      <button
        className={clsx('button', sizeClass, outlineClass, variantClass, blockClass, disabledClass, className)}
        style={style}
        role="button"
        aria-disabled={disabled}
      >
        {parsedLabel}
      </button>
    </Link>
  );
};

export default InstallButton;
