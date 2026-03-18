import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../ui/dialog';
import { useTranslation } from 'react-i18next';

interface PrivacyInfoModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function PrivacyInfoModal({ isOpen, onClose }: PrivacyInfoModalProps) {
  const { t } = useTranslation();

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="w-[440px]">
        <DialogHeader>
          <DialogTitle className="text-center">{t('privacy.title')}</DialogTitle>
        </DialogHeader>

        <div>
          <p className="text-text-muted text-sm mb-3">
            {t('privacy.description')}
          </p>
          <p className="font-medium text-text-default text-sm mb-1.5">{t('privacy.whatWeCollectTitle')}</p>
          <ul className="text-text-muted text-sm list-disc list-outside space-y-0.5 ml-5 mb-3">
            <li>{t('telemetry.collectItems.os')}</li>
            <li>{t('telemetry.collectItems.version')}</li>
            <li>{t('telemetry.collectItems.provider')}</li>
            <li>{t('telemetry.collectItems.extensions')}</li>
            <li>{t('telemetry.collectItems.sessionMetrics')}</li>
            <li>{t('telemetry.collectItems.errorTypes')}</li>
          </ul>
          <p className="text-text-muted text-sm">
            {t('privacy.privacyNote')}
          </p>
        </div>
      </DialogContent>
    </Dialog>
  );
}
