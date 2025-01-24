import { StylesConfig, ThemeConfig } from 'react-select';

export const createDarkSelectStyles = (minWidth?: string): StylesConfig => ({
  control: (base) => ({
    ...base,
    ...(minWidth ? { minWidth } : {}),
    backgroundColor: '#1a1b1e',
    borderColor: '#2a2b2e',
    color: '#ffffff',
  }),
  menu: (base) => ({
    ...base,
    backgroundColor: '#1a1b1e',
    boxShadow: '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
    border: '1px solid #2a2b2e',
    background: '#1a1b1e',
  }),
  menuList: (base) => ({
    ...base,
    backgroundColor: '#1a1b1e',
    background: '#1a1b1e',
    padding: '4px',
  }),
  option: (base, state) => ({
    ...base,
    backgroundColor: state.isFocused ? '#2a2b2e' : '#1a1b1e',
    color: '#ffffff',
    cursor: 'pointer',
    background: state.isFocused ? '#2a2b2e' : '#1a1b1e',
    ':hover': {
      backgroundColor: '#2a2b2e',
      color: '#ffffff',
      background: '#2a2b2e',
    },
    padding: '8px',
    margin: '2px 0',
    borderRadius: '4px',
  }),
  singleValue: (base) => ({
    ...base,
    color: '#ffffff',
  }),
  input: (base) => ({
    ...base,
    color: '#ffffff',
  }),
  placeholder: (base) => ({
    ...base,
    color: '#9ca3af',
  }),
  dropdownIndicator: (base) => ({
    ...base,
    color: '#9ca3af',
    ':hover': {
      color: '#ffffff',
    },
  }),
  indicatorSeparator: (base) => ({
    ...base,
    backgroundColor: '#2a2b2e',
  }),
});

export const darkSelectTheme: ThemeConfig = (theme) => ({
  ...theme,
  colors: {
    ...theme.colors,
    primary: '#2a2b2e',
    primary75: '#2a2b2e',
    primary50: '#2a2b2e',
    primary25: '#2a2b2e',
    neutral0: '#1a1b1e',
    neutral5: '#1a1b1e',
    neutral10: '#2a2b2e',
    neutral20: '#2a2b2e',
    neutral30: '#3a3b3e',
    neutral40: '#ffffff',
    neutral50: '#ffffff',
    neutral60: '#ffffff',
    neutral70: '#ffffff',
    neutral80: '#ffffff',
    neutral90: '#ffffff',
  },
});
