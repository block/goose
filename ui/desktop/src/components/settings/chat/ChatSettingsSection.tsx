import { ModeSection } from '../mode/ModeSection';
import { DictationSettings } from '../dictation/DictationSettings';
import { SecurityToggle } from '../security/SecurityToggle';
import { ResponseStylesSection } from '../response_styles/ResponseStylesSection';
import { GoosehintsSection } from './GoosehintsSection';
import { SpellcheckToggle } from './SpellcheckToggle';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';

export default function ChatSettingsSection() {
  return (
    <div className="space-y-4">
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
          <GoosehintsSection />
        </CardContent>
      </Card>

      <Card className="pb-2 rounded-lg">
        <CardContent className="px-2">
          <DictationSettings />
          <SpellcheckToggle />
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
          <SecurityToggle />
        </CardContent>
      </Card>
    </div>
  );
}
