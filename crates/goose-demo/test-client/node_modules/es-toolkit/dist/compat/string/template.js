'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const escape = require('./escape.js');
const attempt = require('../function/attempt.js');
const defaults = require('../object/defaults.js');
const toString = require('../util/toString.js');

const esTemplateRegExp = /\$\{([^\\}]*(?:\\.[^\\}]*)*)\}/g;
const unEscapedRegExp = /['\n\r\u2028\u2029\\]/g;
const noMatchExp = /($^)/;
const escapeMap = new Map([
    ['\\', '\\'],
    ["'", "'"],
    ['\n', 'n'],
    ['\r', 'r'],
    ['\u2028', 'u2028'],
    ['\u2029', 'u2029'],
]);
function escapeString(match) {
    return `\\${escapeMap.get(match)}`;
}
const defaultInterpolateRegExp = /<%=([\s\S]+?)%>/g;
const templateSettings = {
    escape: /<%-([\s\S]+?)%>/g,
    evaluate: /<%([\s\S]+?)%>/g,
    interpolate: defaultInterpolateRegExp,
    variable: '',
    imports: {
        _: {
            escape: escape.escape,
            template,
        },
    },
};
function template(string, options, guard) {
    string = toString.toString(string);
    if (guard) {
        options = templateSettings;
    }
    options = defaults.defaults({ ...options }, templateSettings);
    const delimitersRegExp = new RegExp([
        options.escape?.source ?? noMatchExp.source,
        options.interpolate?.source ?? noMatchExp.source,
        options.interpolate === defaultInterpolateRegExp ? esTemplateRegExp.source : noMatchExp.source,
        options.evaluate?.source ?? noMatchExp.source,
        '$',
    ].join('|'), 'g');
    let lastIndex = 0;
    let isEvaluated = false;
    let source = `__p += ''`;
    for (const match of string.matchAll(delimitersRegExp)) {
        const [fullMatch, escapeValue, interpolateValue, esTemplateValue, evaluateValue] = match;
        const { index } = match;
        source += ` + '${string.slice(lastIndex, index).replace(unEscapedRegExp, escapeString)}'`;
        if (escapeValue) {
            source += ` + _.escape(${escapeValue})`;
        }
        if (interpolateValue) {
            source += ` + ((${interpolateValue}) == null ? '' : ${interpolateValue})`;
        }
        else if (esTemplateValue) {
            source += ` + ((${esTemplateValue}) == null ? '' : ${esTemplateValue})`;
        }
        if (evaluateValue) {
            source += `;\n${evaluateValue};\n __p += ''`;
            isEvaluated = true;
        }
        lastIndex = index + fullMatch.length;
    }
    const imports = defaults.defaults({ ...options.imports }, templateSettings.imports);
    const importsKeys = Object.keys(imports);
    const importValues = Object.values(imports);
    const sourceURL = `//# sourceURL=${options.sourceURL ? String(options.sourceURL).replace(/[\r\n]/g, ' ') : `es-toolkit.templateSource[${Date.now()}]`}\n`;
    const compiledFunction = `function(${options.variable || 'obj'}) {
    let __p = '';
    ${options.variable ? '' : 'if (obj == null) { obj = {}; }'}
    ${isEvaluated ? `function print() { __p += Array.prototype.join.call(arguments, ''); }` : ''}
    ${options.variable ? source : `with(obj) {\n${source}\n}`}
    return __p;
  }`;
    const result = attempt.attempt(() => new Function(...importsKeys, `${sourceURL}return ${compiledFunction}`)(...importValues));
    result.source = compiledFunction;
    if (result instanceof Error) {
        throw result;
    }
    return result;
}

exports.template = template;
exports.templateSettings = templateSettings;
