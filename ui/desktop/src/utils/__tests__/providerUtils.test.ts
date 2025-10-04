import { describe, it, expect } from 'vitest';
import {
  extractTemplateVariables,
  filterValidUsedParameters,
  substituteParameters,
} from '../providerUtils';
import type { RecipeParameter } from '../../api';

describe('providerUtils', () => {
  describe('extractTemplateVariables', () => {
    it('should extract simple template variables', () => {
      const content = 'Hello {{name}}, welcome to {{app}}!';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['name', 'app']);
    });

    it('should extract variables with underscores', () => {
      const content = 'User: {{user_name}}, ID: {{user_id}}';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['user_name', 'user_id']);
    });

    it('should extract variables that start with underscore', () => {
      const content = 'Private: {{_private}}, Internal: {{__internal}}';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['_private', '__internal']);
    });

    it('should handle variables with numbers', () => {
      const content = 'Item {{item1}}, Version {{version2_0}}';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['item1', 'version2_0']);
    });

    it('should trim whitespace from variables', () => {
      const content = 'Hello {{ name }}, welcome to {{  app  }}!';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['name', 'app']);
    });

    it('should ignore invalid variable names with spaces', () => {
      const content = 'Invalid: {{user name}}, Valid: {{username}}';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['username']);
    });

    it('should ignore invalid variable names with dots', () => {
      const content = 'Invalid: {{user.name}}, Valid: {{user_name}}';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['user_name']);
    });

    it('should ignore invalid variable names with pipes', () => {
      const content = 'Invalid: {{name|upper}}, Valid: {{name}}';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['name']);
    });

    it('should ignore invalid variable names with special characters', () => {
      const content = 'Invalid: {{user@name}}, {{user-name}}, {{user$name}}, Valid: {{username}}';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['username']);
    });

    it('should ignore variables starting with numbers', () => {
      const content = 'Invalid: {{1name}}, {{2user}}, Valid: {{name1}}';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['name1']);
    });

    it('should remove duplicates', () => {
      const content = 'Hello {{name}}, goodbye {{name}}, welcome {{app}}, use {{app}}';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['name', 'app']);
    });

    it('should handle empty content', () => {
      const content = '';
      const result = extractTemplateVariables(content);
      expect(result).toEqual([]);
    });

    it('should handle content with no variables', () => {
      const content = 'This is just plain text with no variables.';
      const result = extractTemplateVariables(content);
      expect(result).toEqual([]);
    });

    it('should handle single braces (not template variables)', () => {
      const content = 'This {is} not a {template} variable but {{this}} is.';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['this']);
    });

    it('should handle malformed template syntax', () => {
      const content = 'Malformed: {{{name}}}, {{name}}, {name}}';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['name']);
    });

    it('should handle empty variable names', () => {
      const content = 'Empty: {{}}, Valid: {{name}}';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['name']);
    });

    it('should handle variables with only whitespace', () => {
      const content = 'Whitespace: {{   }}, Valid: {{name}}';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['name']);
    });

    it('should ignore complex template expressions with dots and pipes', () => {
      const content =
        'Complex: {{steps.fetch_payment_data.data.payments.totalEdgeCount | number_format}}, Valid: {{simple_param}}';
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['simple_param']);
    });

    it('should handle complex mixed content', () => {
      const content = `
        Welcome {{user_name}}!
        
        Your account details:
        - ID: {{user_id}}
        - Email: {{email_address}}
        - Invalid: {{user.email}}
        - Invalid: {{user name}}
        - Invalid: {{1invalid}}
        
        Thank you for using {{app_name}}!
      `;
      const result = extractTemplateVariables(content);
      expect(result).toEqual(['user_name', 'user_id', 'email_address', 'app_name']);
    });
  });

  describe('filterValidUsedParameters', () => {
    const createParameter = (
      key: string,
      description = '',
      requirement: 'required' | 'optional' | 'user_prompt' = 'optional'
    ): RecipeParameter => ({
      key,
      description,
      input_type: 'string',
      requirement,
    });

    it('should filter parameters to only include valid ones used in content', () => {
      const parameters = [
        createParameter('valid_param'),
        createParameter('invalid param'), // has space
        createParameter('unused_param'),
        createParameter('used_param'),
      ];

      const recipeContent = {
        prompt: 'Use {{valid_param}} and {{used_param}}',
        instructions: 'Additional {{valid_param}} usage',
      };

      const result = filterValidUsedParameters(parameters, recipeContent);
      expect(result).toEqual([createParameter('valid_param'), createParameter('used_param')]);
    });

    it('should handle parameters used only in prompt', () => {
      const parameters = [createParameter('prompt_param'), createParameter('unused_param')];

      const recipeContent = {
        prompt: 'Use {{prompt_param}}',
        instructions: 'No parameters here',
      };

      const result = filterValidUsedParameters(parameters, recipeContent);
      expect(result).toEqual([createParameter('prompt_param')]);
    });

    it('should handle parameters used only in instructions', () => {
      const parameters = [createParameter('instruction_param'), createParameter('unused_param')];

      const recipeContent = {
        prompt: 'No parameters here',
        instructions: 'Use {{instruction_param}}',
      };

      const result = filterValidUsedParameters(parameters, recipeContent);
      expect(result).toEqual([createParameter('instruction_param')]);
    });

    it('should remove duplicate parameters (keep first occurrence)', () => {
      const parameters = [
        createParameter('duplicate_param', 'First occurrence'),
        createParameter('duplicate_param', 'Second occurrence'),
        createParameter('unique_param'),
      ];

      const recipeContent = {
        prompt: 'Use {{duplicate_param}} and {{unique_param}}',
      };

      const result = filterValidUsedParameters(parameters, recipeContent);
      expect(result).toEqual([
        createParameter('duplicate_param', 'First occurrence'),
        createParameter('unique_param'),
      ]);
    });

    it('should filter out parameters with invalid names', () => {
      const parameters = [
        createParameter('valid_param'),
        createParameter('invalid param'), // space
        createParameter('invalid.param'), // dot
        createParameter('invalid|param'), // pipe
        createParameter('invalid-param'), // dash
        createParameter('invalid@param'), // at symbol
        createParameter('1invalid'), // starts with number
        createParameter('_valid_param'), // starts with underscore (valid)
      ];

      const recipeContent = {
        prompt:
          'Use all: {{valid_param}} {{invalid param}} {{invalid.param}} {{invalid|param}} {{invalid-param}} {{invalid@param}} {{1invalid}} {{_valid_param}}',
      };

      const result = filterValidUsedParameters(parameters, recipeContent);
      expect(result).toEqual([createParameter('valid_param'), createParameter('_valid_param')]);
    });

    it('should handle empty parameters array', () => {
      const parameters: RecipeParameter[] = [];
      const recipeContent = {
        prompt: 'Use {{some_param}}',
      };

      const result = filterValidUsedParameters(parameters, recipeContent);
      expect(result).toEqual([]);
    });

    it('should handle undefined parameters', () => {
      const parameters = undefined;
      const recipeContent = {
        prompt: 'Use {{some_param}}',
      };

      const result = filterValidUsedParameters(parameters, recipeContent);
      expect(result).toEqual([]);
    });

    it('should handle empty recipe content', () => {
      const parameters = [createParameter('param1'), createParameter('param2')];
      const recipeContent = {};

      const result = filterValidUsedParameters(parameters, recipeContent);
      expect(result).toEqual([]);
    });

    it('should handle recipe content with empty strings', () => {
      const parameters = [createParameter('param1'), createParameter('param2')];
      const recipeContent = {
        prompt: '',
        instructions: '',
      };

      const result = filterValidUsedParameters(parameters, recipeContent);
      expect(result).toEqual([]);
    });

    it('should handle recipe content with undefined values', () => {
      const parameters = [createParameter('param1'), createParameter('param2')];
      const recipeContent = {
        prompt: undefined,
        instructions: undefined,
      };

      const result = filterValidUsedParameters(parameters, recipeContent);
      expect(result).toEqual([]);
    });

    it('should preserve parameter properties', () => {
      const parameters = [
        {
          key: 'test_param',
          description: 'A test parameter',
          input_type: 'string' as const,
          requirement: 'required' as const,
        },
      ];

      const recipeContent = {
        prompt: 'Use {{test_param}}',
      };

      const result = filterValidUsedParameters(parameters, recipeContent);
      expect(result).toEqual([
        {
          key: 'test_param',
          description: 'A test parameter',
          input_type: 'string',
          requirement: 'required',
        },
      ]);
    });

    it('should filter out complex template expressions with dots and pipes', () => {
      const parameters = [
        createParameter('steps.fetch_payment_data.data.payments.totalEdgeCount | number_format'), // complex invalid
        createParameter('simple_param'), // valid
        createParameter('another.invalid.param'), // invalid with dots
        createParameter('valid_param'), // valid
      ];

      const recipeContent = {
        prompt:
          'Use {{steps.fetch_payment_data.data.payments.totalEdgeCount | number_format}} and {{simple_param}}',
        instructions: 'Also use {{another.invalid.param}} and {{valid_param}}',
      };

      const result = filterValidUsedParameters(parameters, recipeContent);
      expect(result).toEqual([createParameter('simple_param'), createParameter('valid_param')]);
    });

    it('should handle complex recipe content with multiple parameter usages', () => {
      const parameters = [
        createParameter('user_name'),
        createParameter('user_email'),
        createParameter('app_name'),
        createParameter('invalid param'),
        createParameter('unused_param'),
        createParameter('version_number'),
      ];

      const recipeContent = {
        prompt: `
          Welcome {{user_name}}!
          
          Your details:
          - Name: {{user_name}}
          - Email: {{user_email}}
        `,
        instructions: `
          Please use {{app_name}} version {{version_number}}.
          
          Contact {{user_email}} for support.
          Invalid usage: {{invalid param}}
        `,
      };

      const result = filterValidUsedParameters(parameters, recipeContent);
      expect(result).toEqual([
        createParameter('user_name'),
        createParameter('user_email'),
        createParameter('app_name'),
        createParameter('version_number'),
      ]);
    });
  });

  describe('substituteParameters', () => {
    it('should substitute simple parameters', () => {
      const text = 'Hello {{name}}, welcome to {{app}}!';
      const params = { name: 'John', app: 'MyApp' };
      const result = substituteParameters(text, params);
      expect(result).toBe('Hello John, welcome to MyApp!');
    });

    it('should handle parameters with underscores', () => {
      const text = 'User: {{user_name}}, ID: {{user_id}}';
      const params = { user_name: 'john_doe', user_id: '12345' };
      const result = substituteParameters(text, params);
      expect(result).toBe('User: john_doe, ID: 12345');
    });

    it('should handle parameters with whitespace in template', () => {
      const text = 'Hello {{ name }}, welcome to {{  app  }}!';
      const params = { name: 'John', app: 'MyApp' };
      const result = substituteParameters(text, params);
      expect(result).toBe('Hello John, welcome to MyApp!');
    });

    it('should handle multiple occurrences of same parameter', () => {
      const text = 'Hello {{name}}, goodbye {{name}}!';
      const params = { name: 'John' };
      const result = substituteParameters(text, params);
      expect(result).toBe('Hello John, goodbye John!');
    });

    it('should leave unmatched parameters unchanged', () => {
      const text = 'Hello {{name}}, welcome to {{app}}!';
      const params = { name: 'John' }; // missing 'app'
      const result = substituteParameters(text, params);
      expect(result).toBe('Hello John, welcome to {{app}}!');
    });

    it('should handle empty parameters object', () => {
      const text = 'Hello {{name}}, welcome to {{app}}!';
      const params = {};
      const result = substituteParameters(text, params);
      expect(result).toBe('Hello {{name}}, welcome to {{app}}!');
    });

    it('should handle text with no parameters', () => {
      const text = 'This is just plain text.';
      const params = { name: 'John' };
      const result = substituteParameters(text, params);
      expect(result).toBe('This is just plain text.');
    });

    it('should handle empty text', () => {
      const text = '';
      const params = { name: 'John' };
      const result = substituteParameters(text, params);
      expect(result).toBe('');
    });

    it('should handle parameters with special characters in values', () => {
      const text = 'Message: {{message}}';
      const params = { message: 'Hello $world! (test) [array] {object}' };
      const result = substituteParameters(text, params);
      expect(result).toBe('Message: Hello $world! (test) [array] {object}');
    });

    it('should handle parameters with regex special characters in keys', () => {
      const text = 'Value: {{test_param}}';
      const params = { test_param: 'test value' };
      const result = substituteParameters(text, params);
      expect(result).toBe('Value: test value');
    });

    it('should handle parameters with newlines in values', () => {
      const text = 'Content: {{content}}';
      const params = { content: 'Line 1\nLine 2\nLine 3' };
      const result = substituteParameters(text, params);
      expect(result).toBe('Content: Line 1\nLine 2\nLine 3');
    });

    it('should handle complex substitution scenario', () => {
      const text = `
        Welcome {{user_name}}!
        
        Your account details:
        - ID: {{user_id}}
        - Email: {{user_email}}
        - App: {{app_name}}
        
        Thank you for using {{app_name}}!
      `;

      const params = {
        user_name: 'John Doe',
        user_id: '12345',
        user_email: 'john@example.com',
        app_name: 'MyApp',
      };

      const result = substituteParameters(text, params);
      const expected = `
        Welcome John Doe!
        
        Your account details:
        - ID: 12345
        - Email: john@example.com
        - App: MyApp
        
        Thank you for using MyApp!
      `;

      expect(result).toBe(expected);
    });

    it('should handle single braces (not template variables)', () => {
      const text = 'This {is} not a {template} but {{this}} is.';
      const params = { this: 'replaced' };
      const result = substituteParameters(text, params);
      expect(result).toBe('This {is} not a {template} but replaced is.');
    });

    it('should handle malformed template syntax gracefully', () => {
      const text = 'Malformed: {{{name}}}, Normal: {{name}}';
      const params = { name: 'John' };
      const result = substituteParameters(text, params);
      expect(result).toBe('Malformed: {John}, Normal: John');
    });

    it('should handle parameters with numeric values', () => {
      const text = 'Count: {{count}}, Price: {{price}}';
      const params = { count: '5', price: '19.99' };
      const result = substituteParameters(text, params);
      expect(result).toBe('Count: 5, Price: 19.99');
    });

    it('should handle parameters with boolean-like values', () => {
      const text = 'Enabled: {{enabled}}, Active: {{active}}';
      const params = { enabled: 'true', active: 'false' };
      const result = substituteParameters(text, params);
      expect(result).toBe('Enabled: true, Active: false');
    });

    it('should handle parameters with empty string values', () => {
      const text = 'Name: {{name}}, Value: {{value}}';
      const params = { name: '', value: 'test' };
      const result = substituteParameters(text, params);
      expect(result).toBe('Name: , Value: test');
    });
  });
});
