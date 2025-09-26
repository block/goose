import React from 'react';

interface CardContainerProps {
  header: React.ReactNode;
  body: React.ReactNode;
  onClick: () => void;
  grayedOut: boolean;
  testId?: string;
  borderStyle?: 'solid' | 'dashed';
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
  onClick,
  grayedOut = false,
  testId,
  borderStyle = 'solid',
}: CardContainerProps) {
  return (
    <div
      data-testid={testId}
      className={`relative h-full p-[2px] overflow-hidden rounded-[9px] group/card 
                 ${
                   grayedOut
                     ? 'bg-borderSubtle hover:bg-gray-700'
                     : 'bg-borderSubtle hover:bg-transparent hover:duration-300'
                 }`}
      onClick={!grayedOut ? onClick : undefined}
      style={{
        cursor: !grayedOut ? 'pointer' : 'default',
      }}
    >
      {!grayedOut && <GlowingRing />}
      <div
        className={`relative bg-background-default rounded-lg p-3 transition-all duration-200 h-[160px] flex flex-col
                   ${header ? 'justify-between' : 'justify-center'}
                   ${borderStyle === 'dashed' ? 'border-2 border-dashed' : 'border'}
                   ${
                     grayedOut
                       ? 'border-borderSubtle'
                       : 'border-borderSubtle hover:border-borderStandard'
                   }`}
      >
        {header && (
          <div style={{ opacity: grayedOut ? '0.5' : '1' }}>
            <HeaderContainer>{header}</HeaderContainer>
          </div>
        )}

        <div>{body}</div>
      </div>
    </div>
  );
}
