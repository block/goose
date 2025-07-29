import { useState, useEffect } from 'react';
import { Button } from './ui/button';
import { Check } from './icons';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';

const ModalHelpText = () => (
  <div className="text-sm flex-col space-y-4">
    <p>
      .goosehints is a text file used to provide additional context about your project and improve
      the communication with Goose.
    </p>
    <p>
      Please make sure <span className="font-bold">Developer</span> extension is enabled in the
      settings page. This extension is required to use .goosehints. You'll need to restart your
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

const ModalError = ({ error }: { error: Error }) => (
  <div className="text-sm text-textSubtle">
    <div className="text-red-600">Error reading .goosehints file: {JSON.stringify(error)}</div>
  </div>
);

const ModalFileInfo = ({ filePath, found }: { filePath: string; found: boolean }) => (
  <div className="text-sm font-medium">
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

type GoosehintsModalProps = {
  directory: string;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
};

export const GoosehintsModal = ({ directory, setIsGoosehintsModalOpen }: GoosehintsModalProps) => {
  const goosehintsFilePath = `${directory}/.goosehints`;
  const [goosehintsFile, setGoosehintsFile] = useState<string>('');
  const [goosehintsFileFound, setGoosehintsFileFound] = useState<boolean>(false);
  const [goosehintsFileReadError, setGoosehintsFileReadError] = useState<string>('');

  useEffect(() => {
    const fetchGoosehintsFile = async () => {
      try {
        const { file, error, found } = await getGoosehintsFile(goosehintsFilePath);
        setGoosehintsFile(file);
        setGoosehintsFileFound(found);
        // Only set error if file was found but there was an actual read error
        // If file is not found, treat it as creating a new file (no error)
        setGoosehintsFileReadError(found && error ? error : '');
      } catch (error) {
        console.error('Error fetching .goosehints file:', error);
        setGoosehintsFileReadError('Failed to access .goosehints file');
      }
    };
    if (directory) fetchGoosehintsFile();
  }, [directory, goosehintsFilePath]);

  const writeFile = async () => {
    await window.electron.writeFile(goosehintsFilePath, goosehintsFile);
    setIsGoosehintsModalOpen(false);
  };

  const handleClose = () => {
    setIsGoosehintsModalOpen(false);
  };

  return (
    <Dialog open={true} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[80%] sm:max-h-[80%] overflow-auto">
        <DialogHeader>
          <DialogTitle>Configure .goosehints</DialogTitle>
          <DialogDescription>
            Configure your project's .goosehints file to provide additional context to Goose.
          </DialogDescription>
        </DialogHeader>

        <ModalHelpText />

        <div className="py-4">
          {goosehintsFileReadError ? (
            <ModalError error={new Error(goosehintsFileReadError)} />
          ) : (
            <div className="space-y-2">
              <ModalFileInfo filePath={goosehintsFilePath} found={goosehintsFileFound} />
              <textarea
                defaultValue={goosehintsFile}
                autoFocus
                className="w-full h-80 border rounded-md p-2 text-sm resize-none bg-background-default text-textStandard border-borderStandard focus:outline-none"
                onChange={(event) => setGoosehintsFile(event.target.value)}
              />
            </div>
          )}
        </div>

        <DialogFooter className="pt-2">
          <Button variant="outline" onClick={handleClose}>
            Cancel
          </Button>
          <Button onClick={writeFile}>Save</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
