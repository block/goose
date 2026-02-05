const convertToSpaces = (input, spaces = 2) => {
    return input.replace(/^\t+/gm, $1 => ' '.repeat($1.length * spaces));
};
export default convertToSpaces;
