import { toast } from 'react-toastify';
import { ToastError, ToastLoading, ToastSuccess } from '../../settings/models/toasts';

export interface ToastOptions {
  silent?: boolean;
  showEscMessage?: boolean;
  shouldThrow?: boolean;
}

export default class ToastService {
  private silent: boolean = false;
  private showEscMessage: boolean = true;
  private shouldThrow: boolean = false;

  // Create a singleton instance
  private static instance: ToastService;

  public static getInstance(): ToastService {
    if (!ToastService.instance) {
      ToastService.instance = new ToastService();
    }
    return ToastService.instance;
  }

  configure(options: ToastOptions = {}): void {
    if (options.silent !== undefined) this.silent = options.silent;
    if (options.showEscMessage !== undefined) this.showEscMessage = options.showEscMessage;
    if (options.shouldThrow !== undefined) this.shouldThrow = options.shouldThrow;
  }

  error({ title, msg, traceback }: { title: string; msg: string; traceback: string }): void {
    if (!this.silent) {
      ToastError({ title, msg, traceback });
    }
    console.error(msg, traceback);

    if (this.shouldThrow) {
      throw new Error(msg);
    }
  }

  loading({ title, msg }: { title: string; msg: string }): string | number | undefined {
    if (this.silent) return undefined;

    const toastId = ToastLoading({ title, msg });

    if (this.showEscMessage) {
      toast.info(
        'Press the ESC key on your keyboard to continue using goose while extension loads'
      );
    }
    return toastId;
  }

  success({ title, msg }: { title: string; msg: string }): void {
    if (this.silent) return;
    ToastSuccess({ title, msg });
  }

  dismiss(toastId?: string | number): void {
    if (toastId) toast.dismiss(toastId);
  }

  /**
   * Handle errors with consistent logging and toast notifications
   * Consolidates the functionality of the original handleError function
   */
  handleError(title: string, message: string, options: ToastOptions = {}): void {
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
