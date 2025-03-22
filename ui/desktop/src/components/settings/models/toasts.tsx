import { toast, ToastOptions } from 'react-toastify';
import React from 'react';

const commonToastOptions: ToastOptions = {
  position: 'top-right',
  closeButton: false,
  hideProgressBar: true,
  closeOnClick: true,
  pauseOnHover: true,
  draggable: true,
};

type ToastSuccessProps = { title?: string; msg?: string; toastOptions?: ToastOptions };
export function ToastSuccess({ title, msg, toastOptions = {} }: ToastSuccessProps) {
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
  errorMessage?: string;
  toastOptions?: ToastOptions;
};
export function ToastError({ title, msg, errorMessage, toastOptions }: ToastErrorProps) {
  return toast.error(
    <div className="flex gap-4">
      <div className="flex-grow">
        {title ? <strong className="font-medium">{title}</strong> : null}
        {msg ? <div>{msg}</div> : null}
      </div>
      <div className="flex-none flex items-center">
        {errorMessage ? (
          <button
            className="text-textProminentInverse font-medium"
            onClick={() => navigator.clipboard.writeText(errorMessage)}
          >
            Copy error
          </button>
        ) : null}
      </div>
    </div>,
    { ...commonToastOptions, autoClose: errorMessage ? false : 5000, ...toastOptions }
  );
}

type ToastLoadingProps = { title?: string; msg?: string; toastOptions?: ToastOptions };
export function ToastLoading({ title, msg, toastOptions }: ToastLoadingProps) {
  return toast.loading(
    <div>
      {title ? <strong className="font-medium">{title}</strong> : null}
      {title ? <div>{msg}</div> : null}
    </div>,
    { ...commonToastOptions, autoClose: false, ...toastOptions }
  );
}

type ToastInfoProps = { title?: string; msg?: string; toastOptions?: ToastOptions };
export function ToastInfo({ title, msg, toastOptions }: ToastInfoProps) {
  return toast.info(
    <div>
      {title ? <strong className="font-medium">{title}</strong> : null}
      {msg ? <div>{msg}</div> : null}
    </div>,
    { ...commonToastOptions, ...toastOptions }
  );
}
