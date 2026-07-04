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
import { defineMessages, useIntl } from '../../../i18n';

const i18n = defineMessages({
  dialogTitle: {
    id: 'goslinghintsModal.dialogTitle',
    defaultMessage: 'Configure Project Hints (.goslinghints)',
  },
  dialogDescription: {
    id: 'goslinghintsModal.dialogDescription',
    defaultMessage:
      'Provide additional context about your project to improve communication with Gosling',
  },
  helpText1: {
    id: 'goslinghintsModal.helpText1',
    defaultMessage:
      '.goslinghints is a text file used to provide additional context about your project and improve the communication with Gosling.',
  },
  helpText2: {
    id: 'goslinghintsModal.helpText2',
    defaultMessage:
      "Please make sure {bold} extension is enabled in the extensions page. This extension is required to use .goslinghints. You'll need to restart your session for .goslinghints updates to take effect.",
  },
  helpText3: {
    id: 'goslinghintsModal.helpText3',
    defaultMessage: 'See {link} for more information.',
  },
  helpTextLink: {
    id: 'goslinghintsModal.helpTextLink',
    defaultMessage: 'using .goslinghints',
  },
  errorReading: {
    id: 'goslinghintsModal.errorReading',
    defaultMessage: 'Error reading .goslinghints file: {error}',
  },
  fileFound: {
    id: 'goslinghintsModal.fileFound',
    defaultMessage: '.goslinghints file found at: {filePath}',
  },
  fileCreating: {
    id: 'goslinghintsModal.fileCreating',
    defaultMessage: 'Creating new .goslinghints file at: {filePath}',
  },
  placeholder: {
    id: 'goslinghintsModal.placeholder',
    defaultMessage: 'Enter project hints here...',
  },
  savedSuccessfully: {
    id: 'goslinghintsModal.savedSuccessfully',
    defaultMessage: 'Saved successfully',
  },
  close: {
    id: 'goslinghintsModal.close',
    defaultMessage: 'Close',
  },
  saving: {
    id: 'goslinghintsModal.saving',
    defaultMessage: 'Saving...',
  },
  save: {
    id: 'goslinghintsModal.save',
    defaultMessage: 'Save',
  },
  failedToAccess: {
    id: 'goslinghintsModal.failedToAccess',
    defaultMessage: 'Failed to access .goslinghints file',
  },
  failedToSave: {
    id: 'goslinghintsModal.failedToSave',
    defaultMessage: 'Failed to save .goslinghints file',
  },
  developer: {
    id: 'goslinghintsModal.developer',
    defaultMessage: 'Developer',
  },
});

const HelpText = () => {
  const intl = useIntl();

  return (
    <div className="text-sm flex-col space-y-4 text-text-secondary">
      <p>{intl.formatMessage(i18n.helpText1)}</p>
      <p>
        {intl.formatMessage(i18n.helpText2, {
          bold: <span className="font-bold">{intl.formatMessage(i18n.developer)}</span>,
        })}
      </p>
      <p>
        {intl.formatMessage(i18n.helpText3, {
          link: (
            <Button
              variant="link"
              className="text-blue-500 hover:text-blue-600 p-0 h-auto"
              onClick={() =>
                window.open(
                  'https://gosling-docs.ai/docs/guides/context-engineering/using-goslinghints',
                  '_blank'
                )
              }
            >
              {intl.formatMessage(i18n.helpTextLink)}
            </Button>
          ),
        })}
      </p>
    </div>
  );
};

const ErrorDisplay = ({ error }: { error: Error }) => {
  const intl = useIntl();

  return (
    <div className="text-sm text-text-secondary">
      <div className="text-red-600">
        {intl.formatMessage(i18n.errorReading, { error: errorMessage(error) })}
      </div>
    </div>
  );
};

const FileInfo = ({ filePath, found }: { filePath: string; found: boolean }) => {
  const intl = useIntl();

  return (
    <div className="text-sm font-medium mb-2">
      {found ? (
        <div className="text-green-600">
          <Check className="w-4 h-4 inline-block" />{' '}
          {intl.formatMessage(i18n.fileFound, { filePath })}
        </div>
      ) : (
        <div>{intl.formatMessage(i18n.fileCreating, { filePath })}</div>
      )}
    </div>
  );
};

const getGoslinghintsFile = async (filePath: string) => await window.electron.readFile(filePath);

interface GoslinghintsModalProps {
  directory: string;
  setIsGoslinghintsModalOpen: (isOpen: boolean) => void;
}

export const GoslinghintsModal = ({ directory, setIsGoslinghintsModalOpen }: GoslinghintsModalProps) => {
  const intl = useIntl();
  const goslinghintsFilePath = `${directory}/.goslinghints`;
  const [goslinghintsFile, setGoslinghintsFile] = useState<string>('');
  const [goslinghintsFileFound, setGoslinghintsFileFound] = useState<boolean>(false);
  const [goslinghintsFileReadError, setGoslinghintsFileReadError] = useState<string>('');
  const [isSaving, setIsSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);

  useEffect(() => {
    const fetchGoslinghintsFile = async () => {
      try {
        const { file, error, found } = await getGoslinghintsFile(goslinghintsFilePath);
        setGoslinghintsFile(file);
        setGoslinghintsFileFound(found);
        setGoslinghintsFileReadError(found && error ? error : '');
      } catch (error) {
        console.error('Error fetching .goslinghints file:', error);
        setGoslinghintsFileReadError(intl.formatMessage(i18n.failedToAccess));
      }
    };
    if (directory) fetchGoslinghintsFile();
  }, [directory, goslinghintsFilePath, intl]);

  const writeFile = async () => {
    setIsSaving(true);
    setSaveSuccess(false);
    try {
      await window.electron.writeFile(goslinghintsFilePath, goslinghintsFile);
      setSaveSuccess(true);
      setGoslinghintsFileFound(true);
      setTimeout(() => setSaveSuccess(false), 3000);
    } catch (error) {
      console.error('Error writing .goslinghints file:', error);
      setGoslinghintsFileReadError(intl.formatMessage(i18n.failedToSave));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <Dialog open={true} onOpenChange={(open) => setIsGoslinghintsModalOpen(open)}>
      <DialogContent className="w-[80vw] max-w-[80vw] sm:max-w-[80vw] max-h-[90vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>{intl.formatMessage(i18n.dialogTitle)}</DialogTitle>
          <DialogDescription>{intl.formatMessage(i18n.dialogDescription)}</DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto space-y-4 pt-2 pb-4">
          <HelpText />

          <div>
            {goslinghintsFileReadError ? (
              <ErrorDisplay error={new Error(goslinghintsFileReadError)} />
            ) : (
              <div className="space-y-2">
                <FileInfo filePath={goslinghintsFilePath} found={goslinghintsFileFound} />
                <textarea
                  value={goslinghintsFile}
                  className="w-full h-80 border rounded-md p-2 text-sm resize-none bg-background-primary text-text-primary border-border-primary focus:outline-none focus:ring-2 focus:ring-blue-500"
                  onChange={(event) => setGoslinghintsFile(event.target.value)}
                  placeholder={intl.formatMessage(i18n.placeholder)}
                />
              </div>
            )}
          </div>
        </div>

        <DialogFooter>
          {saveSuccess && (
            <span className="text-green-600 text-sm flex items-center gap-1 mr-auto">
              <Check className="w-4 h-4" />
              {intl.formatMessage(i18n.savedSuccessfully)}
            </span>
          )}
          <Button variant="outline" onClick={() => setIsGoslinghintsModalOpen(false)}>
            {intl.formatMessage(i18n.close)}
          </Button>
          <Button onClick={writeFile} disabled={isSaving}>
            {isSaving ? intl.formatMessage(i18n.saving) : intl.formatMessage(i18n.save)}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
