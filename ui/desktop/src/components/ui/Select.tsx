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
        container: () => 'w-full cursor-pointer relative',
        indicatorSeparator: () => 'h-0',
        control: ({ isFocused }) =>
          `border ${isFocused ? 'border-borderStandard' : 'border-borderSubtle'} focus:border-borderStandard hover:border-borderStandard rounded-md w-full px-2 py-1 text-sm text-textSubtle hover:cursor-pointer`,
        valueContainer: () => 'gap-1 px-2 py-1 flex flex-wrap',
        menu: () =>
          'mt-1 bg-background-default border border-borderStandard rounded-md text-textSubtle shadow-lg select__menu z-[9999] absolute',
        menuList: () => 'max-h-60 overflow-y-auto py-1',
        option: ({ isFocused, isSelected, isDisabled }) => {
          let classes = 'py-2 px-4 text-sm cursor-pointer';

          if (isDisabled) {
            classes += ' opacity-50 cursor-not-allowed pointer-events-none';
          } else if (isSelected) {
            classes += ' bg-background-accent text-text-on-accent pointer-events-auto';
          } else if (isFocused) {
            classes += ' bg-background-muted text-textStandard pointer-events-auto';
          } else {
            classes += ' text-textStandard hover:bg-background-muted pointer-events-auto';
          }

          return classes;
        },
        multiValue: () =>
          'bg-background-muted border border-borderSubtle rounded-md mr-1 mb-1',
        multiValueLabel: () =>
          'px-2 py-1 text-sm text-textStandard',
        multiValueRemove: () =>
          'px-1 py-1 hover:bg-background-accent hover:text-text-on-accent rounded-r-md cursor-pointer',
      }}
      menuShouldBlockScroll={false}
      menuShouldScrollIntoView={false}
      tabSelectsValue={true}
      openMenuOnFocus={false}
      styles={{
        menu: (base) => ({
          ...base,
          pointerEvents: 'auto',
          zIndex: 9999,
        }),
        menuList: (base) => ({
          ...base,
          maxHeight: '240px',
          overflowY: 'auto',
          pointerEvents: 'auto',
        }),
        option: (base) => ({
          ...base,
          pointerEvents: 'auto',
          cursor: 'pointer',
        }),
      }}
    />
  );
};
