import React from 'react';

interface CardContainerProps {
  header: React.ReactNode;
  body: React.ReactNode;
  onClick: () => void;
  grayedOut: boolean;
}

function GlowingRing() {
  return (
    <div
      className={`absolute pointer-events-none w-[260px] h-[260px] top-[-50px] left-[-30px] origin-center 
                            bg-[linear-gradient(45deg,#13BBAF,#FF4F00)] 
                            animate-[rotate_6s_linear_infinite] z-[-1] 
                            opacity-0 group-hover/card:opacity-100`}
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
}: CardContainerProps) {
  return (
    <div
      className={`relative h-full p-[2px] overflow-hidden rounded-[9px] group/card bg-borderSubtle ${
        !grayedOut ? 'hover:bg-transparent hover:duration-300' : ''
      }`}
      onClick={!grayedOut ? onClick : undefined}
      style={{
        cursor: !grayedOut && onClick ? 'pointer' : 'default',
        opacity: !grayedOut ? '1' : '0.5',
      }}
    >
      {!grayedOut && <GlowingRing />}
      <div
        className={`relative bg-bgApp rounded-lg p-3 transition-all duration-200 h-[160px] flex flex-col justify-between ${
          !grayedOut ? 'hover:border-borderStandard' : ''
        }`}
      >
        <HeaderContainer>{header}</HeaderContainer>
        {body}
      </div>
    </div>
  );
}
