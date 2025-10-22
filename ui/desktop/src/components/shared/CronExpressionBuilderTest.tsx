import React, { useState } from 'react';
import { CronExpressionBuilder } from './CronExpressionBuilder';
import { Card } from '../ui/card';

/**
 * Test component for CronExpressionBuilder
 * This can be temporarily added to a route to test the component
 */
export const CronExpressionBuilderTest: React.FC = () => {
  const [cronExpression, setCronExpression] = useState<string>('');
  const [readableExpression, setReadableExpression] = useState<string>('');
  const [isValid, setIsValid] = useState<boolean>(false);

  const handleCronChange = (cron: string, readable: string, valid: boolean) => {
    setCronExpression(cron);
    setReadableExpression(readable);
    setIsValid(valid);
    console.log('Cron changed:', { cron, readable, valid });
  };

  return (
    <div className="p-8 max-w-2xl mx-auto">
      <Card className="p-6">
        <h1 className="text-2xl font-bold mb-4">Cron Expression Builder Test</h1>
        
        <CronExpressionBuilder
          onChange={handleCronChange}
          defaultFrequency="daily"
          defaultTime="09:00"
        />

        <div className="mt-6 p-4 bg-background-medium rounded-lg">
          <h2 className="text-lg font-semibold mb-2">Output:</h2>
          <div className="space-y-2">
            <div>
              <span className="font-medium">Cron Expression:</span>
              <code className="ml-2 px-2 py-1 bg-background-default rounded">
                {cronExpression || 'Not set'}
              </code>
            </div>
            <div>
              <span className="font-medium">Readable:</span>
              <span className="ml-2">{readableExpression || 'Not set'}</span>
            </div>
            <div>
              <span className="font-medium">Valid:</span>
              <span className={`ml-2 ${isValid ? 'text-green-600' : 'text-red-600'}`}>
                {isValid ? '✓ Yes' : '✗ No'}
              </span>
            </div>
          </div>
        </div>

        <div className="mt-4 p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
          <h3 className="font-semibold mb-2">Test Cases:</h3>
          <ul className="text-sm space-y-1 list-disc list-inside">
            <li>Try "Daily" at different times</li>
            <li>Try "Every" with different intervals</li>
            <li>Try "Weekly" and select multiple days</li>
            <li>Try "Monthly" with different days</li>
            <li>Try "Once" with a future date/time</li>
          </ul>
        </div>
      </Card>
    </div>
  );
};

export default CronExpressionBuilderTest;
