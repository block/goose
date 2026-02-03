export const unescapeString = (str: string) => str.replace(/\\n/g, "\n");

export const convertToTitleCase = (str: string) => str.replace(/_|-/g, " ").replace(/\b\w/g, char => char.toUpperCase());
