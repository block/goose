import { useEffect } from 'react';
import { toast } from 'react-toastify';
import { useNavigate } from 'react-router';
import { Button } from './ui/button';
import { defineMessages, useIntl } from '../i18n';

const i18n = defineMessages({
  body: {
    id: 'externalBackendFallback.body',
    defaultMessage:
      'Goose could not connect to the external backend at {url}, so this window started on the local backend.',
  },
  openSettings: {
    id: 'externalBackendFallback.openSettings',
    defaultMessage: 'Open settings',
  },
});

export default function ExternalBackendFallbackNotice() {
  const intl = useIntl();
  const navigate = useNavigate();

  useEffect(() => {
    let cancelled = false;

    void window.electron.takeExternalBackendFallback().then((fallback) => {
      if (cancelled || !fallback) {
        return;
      }

      toast.warning(
        ({ closeToast }) => (
          <div className="flex flex-col gap-2">
            <div className="text-sm">{intl.formatMessage(i18n.body, { url: fallback.url })}</div>
            {fallback.reason && (
              <div className="text-xs opacity-80 break-words">{fallback.reason}</div>
            )}
            <Button
              size="sm"
              className="self-start"
              onClick={() => {
                navigate('/settings?section=external-backend');
                closeToast?.();
              }}
            >
              {intl.formatMessage(i18n.openSettings)}
            </Button>
          </div>
        ),
        { autoClose: false, closeOnClick: false }
      );
    });

    return () => {
      cancelled = true;
    };
  }, [intl, navigate]);

  return null;
}
