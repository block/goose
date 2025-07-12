import React from 'react';
import ReactSelect from 'react-select';

export const Select = (props: React.ComponentProps<typeof ReactSelect>) => {
  return (
    <ReactSelect
      {...props}
      unstyled
      isSearchable={props.isSearchable !== false}
      closeMenuOnSelect={props.closeMenuOnSelect !== false}
      blurInputOnSelect={props.blurInputOnSelect !== false}
      classNames={{
        container: () => 'w-full cursor-pointer relative z-[99999]',
        indicatorSeparator: () => 'h-0',
        control: ({ isFocused }) =>
          `border ${isFocused ? 'border-borderStandard' : 'border-borderSubtle'} focus:border-borderStandard hover:border-borderStandard rounded-md w-full px-4 py-2 text-sm text-textSubtle hover:cursor-pointer`,
        menu: () =>
          'mt-1 bg-background-default border border-borderStandard rounded-md text-textSubtle overflow-hidden absolute z-[99999] select__menu',
        option: ({ isFocused, isSelected, isDisabled }) => {
          let classes = 'py-2 px-4 text-sm cursor-pointer';

          if (isDisabled) {
            classes += ' opacity-50 cursor-not-allowed';
          } else if (isSelected) {
            classes += ' bg-background-accent text-text-on-accent';
          } else if (isFocused) {
            classes += ' bg-background-muted text-textStandard';
          } else {
            classes += ' text-textStandard hover:bg-background-muted';
          }

          return classes;
        },
      }}
      menuShouldBlockScroll={false}
      menuShouldScrollIntoView={true}
      tabSelectsValue={true}
      openMenuOnFocus={false}
    />
  );
};
