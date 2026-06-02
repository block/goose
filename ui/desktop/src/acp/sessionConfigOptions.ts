import type { SessionConfigOption } from '@agentclientprotocol/sdk';
import {
  all_goose_modes,
  type GooseMode,
} from '../components/settings/mode/ModeSelectionItem';

type SelectOption = {
  value: string;
  name: string;
  description?: string | null;
};

type SelectGroup = {
  options: SelectOption[];
};

export function findModeConfigOption(
  configOptions?: SessionConfigOption[] | null
): SessionConfigOption | undefined {
  return configOptions?.find((option) => option.category === 'mode' || option.id === 'mode');
}

export function modeConfigOptionToModes(option?: SessionConfigOption): GooseMode[] | undefined {
  if (!option || option.type !== 'select') {
    return undefined;
  }

  const options = flattenSelectOptions(option.options as Array<SelectOption | SelectGroup>);
  if (options.length === 0) {
    return undefined;
  }

  return options.map((selectOption) => ({
    key: selectOption.value,
    ...displayForModeOption(selectOption),
  }));
}

function displayForModeOption(selectOption: SelectOption): Omit<GooseMode, 'key'> {
  const knownMode = all_goose_modes.find((mode) => mode.key === selectOption.value);
  if (knownMode) {
    return {
      labelDescriptor: knownMode.labelDescriptor,
      descriptionDescriptor: knownMode.descriptionDescriptor,
    };
  }

  return {
    label: selectOption.name,
    description: selectOption.description ?? undefined,
  };
}

function flattenSelectOptions(options: Array<SelectOption | SelectGroup>): SelectOption[] {
  return options.flatMap((option) => {
    if ('options' in option && Array.isArray(option.options)) {
      return option.options;
    }

    return isSelectOption(option) ? [option] : [];
  });
}

function isSelectOption(option: SelectOption | SelectGroup): option is SelectOption {
  return 'value' in option && 'name' in option;
}
