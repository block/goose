import React from 'react';
import { RecentModels } from './RecentModels';
import { ProviderButtons } from './ProviderButtons';
import BackButton from '../../ui/BackButton';
import { SearchBar } from './Search';
import { useModel } from './ModelContext';
import { AddModelInline } from './AddModelInline';
import { ScrollArea } from '../../ui/scroll-area';
import type { View } from '../../../ChatWindow';

export default function MoreModelsPage({
  onClose,
  setView,
}: {
  onClose: () => void;
  setView: (view: View) => void;
}) {
  const { currentModel } = useModel();

  return (
    <div className="h-screen w-full">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

      <ScrollArea className="h-full w-full">
        <div className="px-8 pt-6 pb-4">
          <BackButton onClick={onClose} />
          <h1 className="text-3xl font-medium text-textStandard mt-1">Browse models</h1>
        </div>

        {/* Content Area */}
        <div className="flex-1 py-8 pt-[20px]">
          <div className="max-w-full md:max-w-3xl mx-auto space-y-12">
            <div className="flex justify-between items-center mb-6 border-b border-borderSubtle px-8">
              <h2 className="text-xl font-medium text-textStandard">Models</h2>
              <button
                onClick={() => setView('configureProviders')}
                className="text-indigo-500 hover:text-indigo-600 text-sm"
              >
                Configure
              </button>
            </div>

            <div className="px-8 space-y-8">
              {/* Search Section */}
              <section>
                <h2 className="text-md font-medium text-textStandard mb-3">Search Models</h2>
                <SearchBar />
              </section>

              {/* Add Model Section */}
              <section>
                <h2 className="text-md font-medium text-textStandard mb-3">Add Model</h2>
                <AddModelInline />
              </section>

              {/* Provider Section */}
              <section>
                <h2 className="text-md font-medium text-textStandard mb-3">Browse by Provider</h2>
                <div>
                  <ProviderButtons />
                </div>
              </section>

              {/* Recent Models Section */}
              <section>
                <div className="flex items-center justify-between mb-3">
                  <h2 className="text-md font-medium text-textStandard">Recently used</h2>
                </div>
                <div>
                  <RecentModels />
                </div>
              </section>
            </div>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
