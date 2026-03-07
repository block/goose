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
import { useLocalization } from '../../../contexts/LocalizationContext';

const HelpText = ({
  intro,
  developerExtensionRequired,
  documentationPrefix,
  documentationLink,
  documentationSuffix,
}: {
  intro: string;
  developerExtensionRequired: string;
  documentationPrefix: string;
  documentationLink: string;
  documentationSuffix: string;
}) => (
  <div className="text-sm flex-col space-y-4 text-text-secondary">
    <p>{intro}</p>
    <p>{developerExtensionRequired}</p>
    <p>
      {documentationPrefix}{' '}
      <Button
        variant="link"
        className="text-blue-500 hover:text-blue-600 p-0 h-auto"
        onClick={() =>
          window.open('https://block.github.io/goose/docs/guides/using-goosehints/', '_blank')
        }
      >
        {documentationLink}
      </Button>{' '}
      {documentationSuffix}
    </p>
  </div>
);

const ErrorDisplay = ({ message }: { message: string }) => (
  <div className="text-sm text-text-secondary">
    <div className="text-red-600">{message}</div>
  </div>
);

const FileInfo = ({
  text,
  found,
}: {
  text: string;
  found: boolean;
}) => (
  <div className="text-sm font-medium mb-2">
    {found ? (
      <div className="text-green-600">
        <Check className="w-4 h-4 inline-block" /> {text}
      </div>
    ) : (
      <div>{text}</div>
    )}
  </div>
);

const getGoosehintsFile = async (filePath: string) => await window.electron.readFile(filePath);

interface GoosehintsModalProps {
  directory: string;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}

export const GoosehintsModal = ({ directory, setIsGoosehintsModalOpen }: GoosehintsModalProps) => {
  const { t } = useLocalization();
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
        setGoosehintsFileReadError(t('gooseHintsModal.accessFailed'));
      }
    };
    if (directory) fetchGoosehintsFile();
  }, [directory, goosehintsFilePath, t]);

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
      setGoosehintsFileReadError(t('gooseHintsModal.saveFailed'));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <Dialog open={true} onOpenChange={(open) => setIsGoosehintsModalOpen(open)}>
      <DialogContent className="w-[80vw] max-w-[80vw] sm:max-w-[80vw] max-h-[90vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>{t('gooseHintsModal.title')}</DialogTitle>
          <DialogDescription>{t('gooseHintsModal.description')}</DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto space-y-4 pt-2 pb-4">
          <HelpText
            intro={t('gooseHintsModal.intro')}
            developerExtensionRequired={t('gooseHintsModal.developerExtensionRequired', {
              extension: 'Developer',
            })}
            documentationPrefix={t('gooseHintsModal.documentationPrefix')}
            documentationLink={t('gooseHintsModal.documentationLink')}
            documentationSuffix={t('gooseHintsModal.documentationSuffix')}
          />

          <div>
            {goosehintsFileReadError ? (
              <ErrorDisplay
                message={t('gooseHintsModal.readError', {
                  error: errorMessage(new Error(goosehintsFileReadError)),
                })}
              />
            ) : (
              <div className="space-y-2">
                <FileInfo
                  found={goosehintsFileFound}
                  text={
                    goosehintsFileFound
                      ? t('gooseHintsModal.fileFoundAt', { path: goosehintsFilePath })
                      : t('gooseHintsModal.creatingAt', { path: goosehintsFilePath })
                  }
                />
                <textarea
                  value={goosehintsFile}
                  className="w-full h-80 border rounded-md p-2 text-sm resize-none bg-background-primary text-text-primary border-border-primary focus:outline-none focus:ring-2 focus:ring-blue-500"
                  onChange={(event) => setGoosehintsFile(event.target.value)}
                  placeholder={t('gooseHintsModal.placeholder')}
                />
              </div>
            )}
          </div>
        </div>

        <DialogFooter>
          {saveSuccess && (
            <span className="text-green-600 text-sm flex items-center gap-1 mr-auto">
              <Check className="w-4 h-4" />
              {t('gooseHintsModal.savedSuccessfully')}
            </span>
          )}
          <Button variant="outline" onClick={() => setIsGoosehintsModalOpen(false)}>
            {t('common.actions.close')}
          </Button>
          <Button onClick={writeFile} disabled={isSaving}>
            {isSaving ? t('gooseHintsModal.saving') : t('common.actions.save')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
