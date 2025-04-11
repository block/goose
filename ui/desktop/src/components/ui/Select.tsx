import React from 'react';
import ReactSelect from 'react-select';

export const Select = (props) => {
  return (
    <ReactSelect
      {...props}
      unstyled
      classNames={{
        container: () => 'w-full cursor-pointer relative z-[99999]',
        indicatorSeparator: () => 'h-0',
        control: ({ isFocused }) =>
          `border ${isFocused ? 'border-borderStandard' : 'border-borderSubtle'} focus:border-borderStandard hover:border-borderStandard rounded-md w-full px-4 py-2 text-sm text-textSubtle hover:cursor-pointer`,
        menu: () =>
          'mt-1 bg-bgApp border border-borderStandard rounded-md text-textSubtle overflow-hidden relative z-[99999] select__menu',
        menuPortal: () => 'z-[99999] select__menu',
        option: () => 'py-2 px-4 hover:cursor-pointer hover:bg-bgSubtle text-textStandard text-sm',
      }}
      menuPortalTarget={document.body}
      styles={{
        menuPortal: (base) => ({
          ...base,
          zIndex: 99999,
        }),
      }}
    />
  );
};
