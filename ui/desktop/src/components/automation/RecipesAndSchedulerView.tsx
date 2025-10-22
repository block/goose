import { useState, useEffect } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { FileText, Clock } from 'lucide-react';
import { Button } from '../ui/button';
import RecipesView from '../recipes/RecipesView';
import SchedulesView from '../schedule/SchedulesView';

interface RecipesAndSchedulerViewProps {
  onClose?: () => void;
}

type TabValue = 'recipes' | 'scheduler';

export default function RecipesAndSchedulerView({ onClose }: RecipesAndSchedulerViewProps) {
  const location = useLocation();
  const navigate = useNavigate();
  
  console.log('[RecipesAndSchedulerView] Component mounted');
  console.log('[RecipesAndSchedulerView] Location state:', location.state);
  
  // Determine initial tab from location state or default to recipes
  const initialTab = (location.state as { tab?: TabValue })?.tab || 'recipes';
  const [activeTab, setActiveTab] = useState<TabValue>(initialTab);

  console.log('[RecipesAndSchedulerView] Active tab:', activeTab);

  // Update tab when location state changes (e.g., from deep links or navigation)
  useEffect(() => {
    const stateTab = (location.state as { tab?: TabValue })?.tab;
    if (stateTab && stateTab !== activeTab) {
      console.log('[RecipesAndSchedulerView] Switching tab to:', stateTab);
      setActiveTab(stateTab);
    }
  }, [location.state, activeTab]);

  // Handle tab change
  const handleTabChange = (tab: TabValue) => {
    console.log('[RecipesAndSchedulerView] Tab changed to:', tab);
    setActiveTab(tab);
    navigate('/automation', { state: { ...location.state, tab }, replace: true });
  };

  return (
    <div className="w-full h-full relative">
      {/* Floating tab switcher */}
      <div className="absolute top-6 left-1/2 transform -translate-x-1/2 z-50">
        <div className="flex gap-1 bg-background-medium/95 backdrop-blur-sm rounded-lg border border-border-default shadow-lg p-1">
          <Button
            onClick={() => handleTabChange('recipes')}
            variant={activeTab === 'recipes' ? 'default' : 'ghost'}
            size="sm"
            className="flex items-center gap-2"
          >
            <FileText className="w-4 h-4" />
            Recipes
          </Button>
          <Button
            onClick={() => handleTabChange('scheduler')}
            variant={activeTab === 'scheduler' ? 'default' : 'ghost'}
            size="sm"
            className="flex items-center gap-2"
          >
            <Clock className="w-4 h-4" />
            Scheduler
          </Button>
        </div>
      </div>

      {/* Render active view */}
      <div className="w-full h-full">
        {activeTab === 'recipes' ? (
          <>
            {console.log('[RecipesAndSchedulerView] Rendering RecipesView')}
            <RecipesView />
          </>
        ) : (
          <>
            {console.log('[RecipesAndSchedulerView] Rendering SchedulesView')}
            <SchedulesView onClose={onClose} />
          </>
        )}
      </div>
    </div>
  );
}
