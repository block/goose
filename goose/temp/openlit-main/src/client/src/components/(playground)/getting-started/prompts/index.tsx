import React from 'react';
import { Card, CardContent, CardTitle } from '@/components/ui/card';
import PromptHubHeader from '../../prompt-hub/header';
import getMessage from '@/constants/messages';

export default function PromptsGettingStarted() {
  return (
    <div className="flex flex-col items-center p-8">
      <div className="max-w-4xl w-full">
        <h1 className="text-3xl font-semibold text-center mb-6 text-stone-800 dark:text-stone-200">
          {getMessage().GET_STARTED_WITH_PROMPT_HUB}
        </h1>
        <p className="text-center text-md mb-6 text-stone-500 dark:text-stone-500">
          {getMessage().GET_STARTED_WITH_PROMPT_HUB_DESCRIPTION}
        </p>
        <PromptHubHeader createNew className="grid grid-cols-2 w-full items-center justify-center gap-6 mb-8 [&>*:first-child]:justify-self-end [&>*:nth-child(2)]:justify-self-start" />

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {getMessage().GET_STARTED_WITH_PROMPT_HUB_FEATURE_DETAILS.map((item) => (
            <Card className='border' key={item.title}>
              <CardTitle className='p-6 gap-2 flex items-center text-stone-700 dark:text-stone-300 text-xl'>
                <span>{item.icon}</span>
                <span>{item.title}</span>
              </CardTitle>
              <CardContent>
                <p className="text-stone-500 dark:text-stone-400">
                  {item.description}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    </div>
  );
}
