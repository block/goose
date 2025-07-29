import { ModeSection } from '../mode/ModeSection';
import { ToolSelectionStrategySection } from '../tool_selection_strategy/ToolSelectionStrategySection';
import SchedulerSection from '../scheduler/SchedulerSection';
import DictationSection from '../dictation/DictationSection';
import { ResponseStylesSection } from '../response_styles/ResponseStylesSection';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';

export default function ChatSettingsSection() {
  return (
    <div className="space-y-4 pr-4 pb-8 mt-1">
      <Card className="pb-2 rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="">Mode</CardTitle>
          <CardDescription>Configure how Goose interacts with tools and extensions</CardDescription>
        </CardHeader>
        <CardContent className="px-2">
          <ModeSection />
        </CardContent>
      </Card>

      <Card className="pb-2 rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="">Response Styles</CardTitle>
          <CardDescription>Choose how Goose should format and style its responses</CardDescription>
        </CardHeader>
        <CardContent className="px-2">
          <ResponseStylesSection />
        </CardContent>
      </Card>

      <Card className="pb-2 rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="">Voice Dictation</CardTitle>
          <CardDescription>Configure voice input for messages</CardDescription>
        </CardHeader>
        <CardContent className="px-2">
          <DictationSection />
        </CardContent>
      </Card>

      <Card className="pb-2 rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="">Scheduling Engine</CardTitle>
          <CardDescription>
            Choose which scheduling backend to use for scheduled recipes and tasks
          </CardDescription>
        </CardHeader>
        <CardContent className="px-2">
          <SchedulerSection />
        </CardContent>
      </Card>

      <Card className="pb-2 rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="">Tool Selection Strategy (preview)</CardTitle>
          <CardDescription>
            Configure how Goose selects tools for your requests. Recommended when many extensions
            are enabled. Available only with Claude models served on Databricks for now.
          </CardDescription>
        </CardHeader>
        <CardContent className="px-2">
          <ToolSelectionStrategySection />
        </CardContent>
      </Card>
    </div>
  );
}
