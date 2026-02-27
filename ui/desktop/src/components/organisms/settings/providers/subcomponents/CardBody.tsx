import type React from 'react';

interface CardBodyProps {
  children?: React.ReactNode;
}

export default function CardBody({ children }: CardBodyProps) {
  if (!children) {
    return null;
  }

  return <div className="flex items-center justify-start">{children}</div>;
}
