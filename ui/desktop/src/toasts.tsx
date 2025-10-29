import { toast, ToastOptions } from 'react-toastify';
import { Button } from './components/ui/button';
import { startNewSession } from './sessions';
import { useNavigation } from './hooks/useNavigation';

export interface ToastServiceOptions {
  silent?: boolean;
  shouldThrow?: boolean;
}

class ToastService {
  private silent: boolean = false;
  private shouldThrow: boolean = false;

  // Create a singleton instance
  private static instance: ToastService;

  public static getInstance(): ToastService {
    if (!ToastService.instance) {
      ToastService.instance = new ToastService();
    }
    return ToastService.instance;
  }

  configure(options: ToastServiceOptions = {}): void {
    if (options.silent !== undefined) {
      this.silent = options.silent;
    }

    if (options.shouldThrow !== undefined) {
      this.shouldThrow = options.shouldThrow;
    }
  }

  error(props: ToastErrorProps): void {
    if (!this.silent) {
      toastError(props);
    }

    if (this.shouldThrow) {
      throw new Error(props.msg);
    }
  }

  loading({ title, msg }: { title: string; msg: string }): string | number | undefined {
    if (this.silent) {
      return undefined;
    }

    const toastId = toastLoading({ title, msg });

    return toastId;
  }

  success({ title, msg }: { title: string; msg: string }): void {
    if (this.silent) {
      return;
    }
    toastSuccess({ title, msg });
  }

  dismiss(toastId?: string | number): void {
    if (toastId) toast.dismiss(toastId);
  }

  /**
   * Handle errors with consistent logging and toast notifications
   * Consolidates the functionality of the original handleError function
   */
  handleError(title: string, message: string, options: ToastServiceOptions = {}): void {
    this.configure(options);
    this.error({
      title: title,
      msg: message,
      traceback: message,
    });
  }
}

// Export a singleton instance for use throughout the app
export const toastService = ToastService.getInstance();

const commonToastOptions: ToastOptions = {
  position: 'top-right',
  closeButton: true,
  hideProgressBar: true,
  closeOnClick: true,
  pauseOnHover: true,
  draggable: true,
};

type ToastSuccessProps = { title?: string; msg?: string; toastOptions?: ToastOptions };

export function toastSuccess({ title, msg, toastOptions = {} }: ToastSuccessProps) {
  return toast.success(
    <div>
      {title ? <strong className="font-medium">{title}</strong> : null}
      {title ? <div>{msg}</div> : null}
    </div>,
    { ...commonToastOptions, autoClose: 3000, ...toastOptions }
  );
}

type ToastErrorProps = {
  title: string;
  msg: string;
  traceback?: string;
  recoverHints?: string;
};

function ToastErrorContent({
  title,
  msg,
  traceback,
  recoverHints,
}: Omit<ToastErrorProps, 'setView'>) {
  const setView = useNavigation();
  const showRecovery = recoverHints && setView;

  return (
    <div className="flex gap-4">
      <div className="flex-grow">
        {title && <strong className="font-medium">{title}</strong>}
        {msg && <div>{msg}</div>}
      </div>
      <div className="flex-none flex items-center gap-2">
        {showRecovery ? (
          <Button onClick={() => startNewSession(recoverHints, null, setView)}>Recover</Button>
        ) : traceback ? (
          <Button onClick={() => navigator.clipboard.writeText(traceback)}>Copy error</Button>
        ) : null}
      </div>
    </div>
  );
}

export function toastError({ title, msg, traceback, recoverHints }: ToastErrorProps) {
  return toast.error(
    <ToastErrorContent title={title} msg={msg} traceback={traceback} recoverHints={recoverHints} />,
    { ...commonToastOptions, autoClose: traceback ? false : 5000 }
  );
}

type ToastLoadingProps = {
  title?: string;
  msg?: string;
  toastOptions?: ToastOptions;
};

export function toastLoading({ title, msg, toastOptions }: ToastLoadingProps) {
  return toast.loading(
    <div>
      {title ? <strong className="font-medium">{title}</strong> : null}
      {title ? <div>{msg}</div> : null}
    </div>,
    { ...commonToastOptions, autoClose: false, ...toastOptions }
  );
}
