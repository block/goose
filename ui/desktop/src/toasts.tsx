import { toast, ToastOptions } from 'react-toastify';
import { Button } from './components/ui/button';

export interface ToastServiceOptions {
  silent?: boolean;
  shouldThrow?: boolean;
}

export default class ToastService {
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

  error({ title, msg, traceback }: { title: string; msg: string; traceback: string }): void {
    if (!this.silent) {
      toastError({ title, msg, traceback });
    }

    if (this.shouldThrow) {
      throw new Error(msg);
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
      title: title || 'Error',
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
  title?: string;
  msg?: string;
  traceback?: string;
  toastOptions?: ToastOptions;
};

export function toastError({ title, msg, traceback, toastOptions }: ToastErrorProps) {
  return toast.error(
    <div className="flex gap-4">
      <div className="flex-grow">
        {title ? <strong className="font-medium">{title}</strong> : null}
        {msg ? <div>{msg}</div> : null}
      </div>
      <div className="flex-none flex items-center">
        {traceback ? (
          <Button onClick={() => navigator.clipboard.writeText(traceback)}>Copy error</Button>
        ) : null}
      </div>
    </div>,
    { ...commonToastOptions, autoClose: traceback ? false : 5000, ...toastOptions }
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

type ToastInfoProps = {
  title?: string;
  msg?: string;
  toastOptions?: ToastOptions;
};

export function toastInfo({ title, msg, toastOptions }: ToastInfoProps) {
  return toast.info(
    <div>
      {title ? <strong className="font-medium">{title}</strong> : null}
      {msg ? <div>{msg}</div> : null}
    </div>,
    { ...commonToastOptions, ...toastOptions }
  );
}
