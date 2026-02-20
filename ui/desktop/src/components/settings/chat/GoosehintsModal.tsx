import { useState, useEffect } from 'react';
import { Button } from '../../ui/button';
import { Check } from '../../icons';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../../ui/dialog';
import { errorMessage } from '../../../utils/conversionUtils';

const HelpText = () => (
  <div className="text-sm flex-col space-y-4 text-text-muted">
    <p>
      .goosehints is a text file used to provide additional context about your project and improve
      the communication with Goose.
    </p>
    <p>
      Please make sure <span className="font-bold">Developer</span> extension is enabled in the
      extensions page. This extension is required to use .goosehints. You'll need to restart your
      session for .goosehints updates to take effect.
    </p>
    <p>
      See{' '}
      <Button
        variant="link"
        className="text-blue-500 hover:text-blue-600 p-0 h-auto"
        onClick={() =>
          window.open('https://block.github.io/goose/docs/guides/using-goosehints/', '_blank')
        }
      >
        using .goosehints
      </Button>{' '}
      for more information.
    </p>
  </div>
);

const ErrorDisplay = ({ error }: { error: Error }) => (
  <div className="text-sm text-text-muted">
    <div className="text-red-600">Error reading .goosehints file: {errorMessage(error)}</div>
  </div>
);

const FileInfo = ({ filePath, found }: { filePath: string; found: boolean }) => (
  <div className="text-sm font-medium mb-2">
    {found ? (
      <div className="text-green-600">
        <Check className="w-4 h-4 inline-block" /> .goosehints file found at: {filePath}
      </div>
    ) : (
      <div>Creating new .goosehints file at: {filePath}</div>
    )}
  </div>
);

const getGoosehintsFile = async (filePath: string) => await window.electron.readFile(filePath);

interface GoosehintsModalProps {
  directory: string;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}

export const GoosehintsModal = ({ directory, setIsGoosehintsModalOpen }: GoosehintsModalProps) => {
  const goosehintsFilePath = `${directory}/.goosehints`;
  const [goosehintsFile, setGoosehintsFile] = useState<string>('');
  const [goosehintsFileFound, setGoosehintsFileFound] = useState<boolean>(false);
  const [goosehintsFileReadError, setGoosehintsFileReadError] = useState<string>('');
  const [isSaving, setIsSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);

  useEffect(() => {
    const fetchGoosehintsFile = async () => {
      try {
        const { file, error, found } = await getGoosehintsFile(goosehintsFilePath);
        setGoosehintsFile(file);
        setGoosehintsFileFound(found);
        setGoosehintsFileReadError(found && error ? error : '');
      } catch (error) {
        console.error('Error fetching .goosehints file:', error);
        setGoosehintsFileReadError('Failed to access .goosehints file');
      }
    };
    if (directory) fetchGoosehintsFile();
  }, [directory, goosehintsFilePath]);

  const writeFile = async () => {
    setIsSaving(true);
    setSaveSuccess(false);
    try {
      await window.electron.writeFile(goosehintsFilePath, goosehintsFile);
      setSaveSuccess(true);
      setGoosehintsFileFound(true);
      setTimeout(() => setSaveSuccess(false), 3000);
    } catch (error) {
      console.error('Error writing .goosehints file:', error);
      setGoosehintsFileReadError('Failed to save .goosehints file');
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <Dialog open={true} onOpenChange={(open) => setIsGoosehintsModalOpen(open)}>
      <DialogContent className="w-[80vw] max-w-[80vw] sm:max-w-[80vw] max-h-[90vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>Configure Project Hints (.goosehints)</DialogTitle>
          <DialogDescription>
            Provide additional context about your project to improve communication with Goose
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto space-y-4 pt-2 pb-4">
          <HelpText />

          <div>
            {goosehintsFileReadError ? (
              <ErrorDisplay error={new Error(goosehintsFileReadError)} />
            ) : (
              <div className="space-y-2">
                <FileInfo filePath={goosehintsFilePath} found={goosehintsFileFound} />
                <textarea
                  value={goosehintsFile}
                  className="w-full h-80 border rounded-md p-2 text-sm resize-none bg-background-default text-text-default border-border-default focus:outline-none focus:ring-2 focus:ring-blue-500"
                  onChange={(event) => setGoosehintsFile(event.target.value)}
                  placeholder="Enter project hints here..."
                />
              </div>
            )}
          </div>
        </div>

        <DialogFooter>
          {saveSuccess && (
            <span className="text-green-600 text-sm flex items-center gap-1 mr-auto">
              <Check className="w-4 h-4" />
              Saved successfully
            </span>
          )}
          <Button variant="outline" onClick={() => setIsGoosehintsModalOpen(false)}>
            Close
          </Button>
          <Button onClick={writeFile} disabled={isSaving}>
            {isSaving ? 'Saving...' : 'Save'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
