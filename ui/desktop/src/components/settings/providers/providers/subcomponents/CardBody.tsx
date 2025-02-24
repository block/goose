import React from 'react';
import CardActions from './CardActions';
import ConfigurationAction from '../interfaces/ConfigurationAction';

interface CardBodyProps {
  actions: ConfigurationAction[];
}

export default function CardBody({ actions }: CardBodyProps) {
  console.log('in card body');
  return (
    <div className="space-x-2 text-center flex items-center justify-between">
      <CardActions actions={actions} />
    </div>
  );
}
