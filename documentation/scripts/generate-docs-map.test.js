const { test, describe } = require('node:test');
const assert = require('node:assert');
const { getTitle, getHeadings } = require('./generate-docs-map');

describe('getTitle', () => {
  test('returns frontmatter title when present', () => {
    const result = getTitle({ title: 'My Custom Title' }, 'some content');
    assert.strictEqual(result, 'My Custom Title');
  });

  test('extracts H1 heading when no frontmatter title', () => {
    const content = '# Hello World\n\nSome paragraph text';
    const result = getTitle({}, content);
    assert.strictEqual(result, 'Hello World');
  });

  test('returns null when no title found', () => {
    const result = getTitle({}, 'no heading here');
    assert.strictEqual(result, null);
  });

  test('prefers frontmatter over H1', () => {
    const content = '# H1 Title\n\nContent';
    const result = getTitle({ title: 'Frontmatter Title' }, content);
    assert.strictEqual(result, 'Frontmatter Title');
  });
});

describe('getHeadings', () => {
  test('extracts H2 headings', () => {
    const content = '# Title\n\n## Section One\n\nText\n\n## Section Two';
    const result = getHeadings(content);
    assert.strictEqual(result, '* Section One\n* Section Two');
  });

  test('creates nested bullets for H3-H6', () => {
    const content = '## Level 2\n### Level 3\n#### Level 4';
    const result = getHeadings(content);
    assert.strictEqual(result, '* Level 2\n  * Level 3\n    * Level 4');
  });

  test('ignores H1 headings', () => {
    const content = '# Title\n## Real Section';
    const result = getHeadings(content);
    assert.strictEqual(result, '* Real Section');
  });

  test('returns empty string when no headings', () => {
    const content = 'Just some text without any headings';
    const result = getHeadings(content);
    assert.strictEqual(result, '');
  });

  test('handles H6 with correct indentation', () => {
    const content = '## H2\n###### H6';
    const result = getHeadings(content);
    assert.strictEqual(result, '* H2\n        * H6');
  });
});
