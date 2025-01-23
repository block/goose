import React from 'react';
import { Button } from '../../ui/button';
import { RecentModels } from './RecentModels';
import { ProviderButtons } from './ProviderButtons';
import BackButton from '../../ui/BackButton';
import { SearchBar } from './Search';
import { useModel } from './ModelContext';
import { AddModelInline } from './AddModelInline';
import { useNavigate } from 'react-router-dom';
import { ScrollArea } from '../../ui/scroll-area';

export default function MoreModelsPage() {
  const { currentModel } = useModel();
  const navigate = useNavigate();

  return (
    <div className="h-screen w-full pt-[36px]">
      <div className="h-full w-full bg-white dark:bg-gray-800 overflow-hidden p-2 pt-0">
        <ScrollArea className="h-full w-full">
          {/*
            Instead of forcing one row, allow the layout
            to stack vertically on small screens:
          */}
          <div className="flex min-h-full flex-col md:flex-row">
            {/* Left Navigation */}
            <div className="w-full md:w-48 md:border-r border-gray-100 dark:border-gray-700 px-2 pt-2">
              <div className="sticky top-8">
                <BackButton className="mb-4" />
              </div>
            </div>

            {/* Content Area */}
            {/* Smaller / responsive padding so we don't overflow on small screens */}
            <div className="flex-1 px-4 py-8 pt-[20px] md:px-8">
              {/*
                Use a max-w but allow full width on very small screens
                so it doesn't overflow horizontally:
              */}
              <div className="max-w-full md:max-w-3xl mx-auto space-y-12">
                {/* Header Section */}
                <div>
                  <div className="flex items-center justify-between mb-8">
                    <h1 className="text-2xl font-semibold tracking-tight">More Models</h1>
                    <Button
                      variant="default"
                      className="h-9 px-4 text-sm whitespace-nowrap shrink-0
                                 bg-gray-800 text-white dark:bg-gray-200 dark:text-gray-900
                                 rounded-full shadow-md border-none
                                 hover:bg-gray-700 hover:text-white
                                 focus:outline-none focus:ring-2 focus:ring-gray-500
                                 dark:hover:bg-gray-300 dark:hover:text-gray-900"
                      onClick={() => navigate('/settings/configure-providers')}
                    >
                      Configure Providers
                    </Button>
                  </div>

                  {currentModel && (
                    <p className="text-sm text-muted-foreground mb-8">
                      Current model: <span className="font-medium">{currentModel.name}</span> (
                      {currentModel.provider})
                    </p>
                  )}
                </div>

                {/* Search Section */}
                <section>
                  <h2 className="text-lg font-medium mb-4">Search Models</h2>
                  <SearchBar />
                </section>

                {/* Add Model Section */}
                <section>
                  <h2 className="text-lg font-medium mb-4">Add Model</h2>
                  <AddModelInline />
                </section>

                {/* Provider Section */}
                <section>
                  <h2 className="text-lg font-medium mb-4">Browse by Provider</h2>
                  <div>
                    <ProviderButtons />
                  </div>
                </section>

                {/* Recent Models Section */}
                <section>
                  <div className="flex items-center justify-between mb-4">
                    <h2 className="text-lg font-medium">Recently Used Models</h2>
                  </div>
                  <div>
                    <RecentModels />
                  </div>
                </section>
              </div>
              {/* end .max-w-full */}
            </div>
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
