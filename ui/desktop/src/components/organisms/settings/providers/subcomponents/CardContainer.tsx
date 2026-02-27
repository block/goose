import type React from 'react';

interface CardContainerProps {
  header: React.ReactNode;
  body: React.ReactNode;
  actions?: React.ReactNode;
  onClick: () => void;
  grayedOut: boolean;
  testId?: string;
  borderStyle?: 'solid' | 'dashed';
  ariaLabel?: string;
}

function GlowingRing() {
  return (
    <div
      className={`absolute pointer-events-none inset-0 rounded-[9px] origin-center 
                            bg-[linear-gradient(45deg,#13BBAF,#FF4F00)] 
                            animate-[rotate_6s_linear_infinite] z-[-1] 
                            opacity-0 group-hover/card:opacity-40 transition-opacity duration-300`}
    />
  );
}

interface HeaderContainerProps {
  children: React.ReactNode;
}

function HeaderContainer({ children }: HeaderContainerProps) {
  return <div>{children}</div>;
}

export default function CardContainer({
  header,
  body,
  actions,
  onClick,
  grayedOut = false,
  testId,
  borderStyle = 'solid',
  ariaLabel,
}: CardContainerProps) {
  const content = (
    <div
      className={`relative bg-background-default rounded-lg p-3 transition-all duration-200 h-[160px] flex flex-col
                 ${header || body ? 'justify-between' : 'justify-center'}
                 ${borderStyle === 'dashed' ? 'border-2 border-dashed' : 'border'}
                 ${
                   grayedOut
                     ? 'border-border-default'
                     : 'border-border-default hover:border-border-default'
                 }`}
    >
      {header && (
        <div style={{ opacity: grayedOut ? '0.5' : '1' }}>
          <HeaderContainer>{header}</HeaderContainer>
        </div>
      )}

      {body && <div>{body}</div>}
    </div>
  );

  const wrapperClassName = `relative h-full p-[2px] overflow-hidden rounded-[9px] group/card
                 ${
                   grayedOut
                     ? 'bg-background-muted hover:bg-gray-700'
                     : 'bg-background-muted hover:bg-transparent hover:duration-300'
                 }`;

  if (grayedOut) {
    return (
      <div data-testid={testId} className={wrapperClassName}>
        {content}
      </div>
    );
  }

  return (
    <div data-testid={testId} className={wrapperClassName}>
      <GlowingRing />
      <button
        type="button"
        className="absolute inset-0 rounded-[9px] focus:outline-none focus:ring-2 focus:ring-border-accent"
        aria-label={ariaLabel ?? 'Open'}
        onClick={onClick}
      />

      <div className="relative z-10">
        <div className="pointer-events-none">{content}</div>
        {actions && <div className="absolute bottom-3 right-3 pointer-events-auto">{actions}</div>}
      </div>
    </div>
  );
}
