import React from 'react';
import CardActions from './CardActions';
import ConfigurationAction from '../interfaces/ConfigurationAction';

interface CardBodyProps {
  children: React.ReactNode;
}

export default function CardBody({ children }: CardBodyProps) {
  return <div className="flex items-center justify-start">{children}</div>;
}
