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
          `border-2 ${isFocused ? 'border-borderStandard' : 'border-borderSubtle'} focus:border-borderStandard hover:border-borderStandard rounded-md w-full px-4 py-2.5 text-sm text-textSubtle`,
        menu: () =>
          'mt-3 bg-bgStandard shadow-xl rounded-md text-textSubtle overflow-hidden relative z-[99999] select__menu',
        menuPortal: () => 'z-[99999] select__menu',
        option: () =>
          'py-4 px-4 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700 text-textProminent font-medium',
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
