import React from 'react';
import CardActions from './CardActions';
import ConfigurationAction from '../interfaces/ConfigurationAction';

interface CardBodyProps {
  actions: ConfigurationAction[];
}

export default function CardBody({ actions }: CardBodyProps) {
  return (
    <div className="flex items-center justify-start">
      <CardActions actions={actions} />
    </div>
  );
}
