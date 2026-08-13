import { useState } from 'react';
import { LogOut } from 'lucide-react';
import { Button } from '../../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../../ui/dialog';
import { acpDeleteProviderConfig } from '../../../acp/providers';
import { toastError } from '../../../toasts';
import { errorMessage } from '../../../utils/conversionUtils';
import { defineMessages, useIntl } from '../../../i18n';

const AVOCADO_PROVIDER = 'avocado';

const i18n = defineMessages({
  title: {
    id: 'signOutSection.title',
    defaultMessage: 'Account',
  },
  description: {
    id: 'signOutSection.description',
    defaultMessage: 'Sign out of Avocado on this device. You will need to sign in again to chat.',
  },
  signOut: {
    id: 'signOutSection.signOut',
    defaultMessage: 'Sign out',
  },
  signingOut: {
    id: 'signOutSection.signingOut',
    defaultMessage: 'Signing out…',
  },
  confirmTitle: {
    id: 'signOutSection.confirmTitle',
    defaultMessage: 'Sign out of Avocado?',
  },
  confirmDescription: {
    id: 'signOutSection.confirmDescription',
    defaultMessage:
      'This removes your Avocado credentials stored on this device. You can sign in again from the welcome screen.',
  },
  cancel: {
    id: 'signOutSection.cancel',
    defaultMessage: 'Cancel',
  },
  failedTitle: {
    id: 'signOutSection.failedTitle',
    defaultMessage: 'Could not sign out',
  },
});

export default function SignOutSection() {
  const intl = useIntl();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [signingOut, setSigningOut] = useState(false);

  const handleSignOut = async () => {
    setSigningOut(true);
    try {
      await acpDeleteProviderConfig(AVOCADO_PROVIDER);
      window.electron.reloadApp();
    } catch (error) {
      console.error('Failed to sign out of Avocado:', error);
      toastError({
        title: intl.formatMessage(i18n.failedTitle),
        msg: errorMessage(error, 'Unknown error'),
      });
      setSigningOut(false);
      setConfirmOpen(false);
    }
  };

  return (
    <>
      <Card className="rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle>{intl.formatMessage(i18n.title)}</CardTitle>
          <CardDescription>{intl.formatMessage(i18n.description)}</CardDescription>
        </CardHeader>
        <CardContent className="pt-4 px-4">
          <Button
            variant="destructive"
            size="sm"
            className="flex items-center gap-2"
            data-testid="avocado-sign-out"
            onClick={() => setConfirmOpen(true)}
            disabled={signingOut}
          >
            <LogOut className="h-4 w-4" />
            {signingOut ? intl.formatMessage(i18n.signingOut) : intl.formatMessage(i18n.signOut)}
          </Button>
        </CardContent>
      </Card>

      <Dialog open={confirmOpen} onOpenChange={(open) => !signingOut && setConfirmOpen(open)}>
        <DialogContent className="sm:max-w-[440px]">
          <DialogHeader>
            <DialogTitle>{intl.formatMessage(i18n.confirmTitle)}</DialogTitle>
            <DialogDescription>{intl.formatMessage(i18n.confirmDescription)}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setConfirmOpen(false)} disabled={signingOut}>
              {intl.formatMessage(i18n.cancel)}
            </Button>
            <Button
              variant="destructive"
              data-testid="avocado-sign-out-confirm"
              onClick={handleSignOut}
              disabled={signingOut}
            >
              {signingOut ? intl.formatMessage(i18n.signingOut) : intl.formatMessage(i18n.signOut)}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
