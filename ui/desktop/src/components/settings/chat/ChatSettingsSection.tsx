import { ModeSection } from '../mode/ModeSection';
import { ToolSelectionStrategySection } from '../tool_selection_strategy/ToolSelectionStrategySection';
import DictationSection from '../dictation/DictationSection';
import { SecurityToggle } from '../security/SecurityToggle';
import { ResponseStylesSection } from '../response_styles/ResponseStylesSection';
import { GoosehintsSection } from './GoosehintsSection';
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
        <CardContent className="px-2">
          <SecurityToggle />
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
        <CardContent className="px-2">
          <DictationSection />
        </CardContent>
      </Card>

      <Card className="pb-2 rounded-lg">
        <CardContent className="px-2">
          <GoosehintsSection />
        </CardContent>
      </Card>

      <Card className="pb-2 rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="">Tool Selection Strategy (preview)</CardTitle>
          <CardDescription>
            Experimental: configure how Goose selects tools for your requests, useful when there are
            many tools. Only tested with Claude models currently.
          </CardDescription>
        </CardHeader>
        <CardContent className="px-2">
          <ToolSelectionStrategySection />
        </CardContent>
      </Card>
    </div>
  );
}
