import { Avocado } from '../icons';
import { Button } from '../ui/button';
import { defineMessages, useIntl } from '../../i18n';

const i18n = defineMessages({
  title: {
    id: 'accessDenied.title',
    defaultMessage: 'Access not granted',
  },
  description: {
    id: 'accessDenied.description',
    defaultMessage:
      'Your account authenticated successfully, but it does not have the agent-access role. Ask an admin to grant access, then try again.',
  },
  switchAccount: {
    id: 'accessDenied.switchAccount',
    defaultMessage: 'Switch account',
  },
  retry: {
    id: 'accessDenied.retry',
    defaultMessage: 'Retry',
  },
});

type AccessDeniedProps = {
  email?: string;
  onSwitchAccount: () => void;
  onRetry: () => void;
};

export function AccessDenied({ email, onSwitchAccount, onRetry }: AccessDeniedProps) {
  const intl = useIntl();
  return (
    <div className="h-screen w-full bg-background-default flex flex-col items-center justify-center">
      <div className="text-center max-w-md px-4">
        <div className="mb-4">
          <Avocado className="size-8 mx-auto" />
        </div>
        <h1 className="text-xl font-light mb-3">{intl.formatMessage(i18n.title)}</h1>
        <p className="text-text-muted mb-2">{intl.formatMessage(i18n.description)}</p>
        {email ? <p className="text-text-muted text-sm mb-6">{email}</p> : <div className="mb-6" />}
        <div className="flex gap-3 justify-center">
          <Button onClick={onSwitchAccount}>{intl.formatMessage(i18n.switchAccount)}</Button>
          <Button variant="outline" onClick={onRetry}>
            {intl.formatMessage(i18n.retry)}
          </Button>
        </div>
      </div>
    </div>
  );
}
