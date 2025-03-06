import { toast } from 'react-toastify';
import React from 'react';
import { Model } from './ModelContext';

export function ToastSuccessModelSwitch(model: Model) {
  return toast.success(
    <div>
      <strong>Model Changed</strong>
      <div>Switched to {model.alias ?? model.name}</div>
    </div>,
    {
      position: 'top-right',
      autoClose: 5000,
      hideProgressBar: true,
      closeOnClick: true,
      pauseOnHover: true,
      draggable: true,
    }
  );
}

export function ToastFailureGeneral(msg?: string) {
  return toast.error(
    <div>
      <strong>Error</strong>
      <div>{msg}</div>
    </div>,
    {
      position: 'top-right',
      autoClose: 3000,
      hideProgressBar: true,
      closeOnClick: true,
      pauseOnHover: true,
      draggable: true,
    }
  );
}
