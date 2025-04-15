import React from 'react';

interface CardBodyProps {
  children: React.ReactNode;
}

export default function CardBody({ children }: CardBodyProps) {
  return <div className="flex items-center justify-start">{children}</div>;
}
