#[cfg(test)]
mod ruby_tests {
    use crate::developer::analyze::parser::{ElementExtractor, ParserManager};

    #[test]
    fn test_ruby_basic_parsing() {
        let parser = ParserManager::new();
        let source = r#"
require 'json'

class MyClass
  attr_accessor :name
  
  def initialize(name)
    @name = name
  end
  
  def greet
    puts "Hello"
  end
end
"#;

        let tree = parser.parse(source, "ruby").unwrap();
        let result = ElementExtractor::extract_elements(&tree, source, "ruby").unwrap();

        // Should find MyClass
        assert_eq!(result.class_count, 1);
        assert!(result.classes.iter().any(|c| c.name == "MyClass"));

        // Should find methods
        assert!(result.function_count > 0);
        assert!(result.functions.iter().any(|f| f.name == "initialize"));
        assert!(result.functions.iter().any(|f| f.name == "greet"));

        // Should find require statement
        assert!(result.import_count > 0);
    }

    #[test]
    fn test_ruby_attr_methods() {
        let parser = ParserManager::new();
        let source = r#"
class Person
  attr_reader :age
  attr_writer :status
  attr_accessor :name
end
"#;

        let tree = parser.parse(source, "ruby").unwrap();
        let result = ElementExtractor::extract_elements(&tree, source, "ruby").unwrap();

        // attr_* should be recognized as functions
        assert!(
            result.function_count >= 3,
            "Expected at least 3 functions from attr_* declarations, got {}",
            result.function_count
        );
    }

    #[test]
    fn test_ruby_require_patterns() {
        let parser = ParserManager::new();
        let source = r#"
require 'json'
require_relative 'lib/helper'
"#;

        let tree = parser.parse(source, "ruby").unwrap();
        let result = ElementExtractor::extract_elements(&tree, source, "ruby").unwrap();

        assert_eq!(
            result.import_count, 2,
            "Should find both require and require_relative"
        );
    }

    #[test]
    fn test_ruby_method_calls() {
        let parser = ParserManager::new();
        let source = r#"
class Example
  def test_method
    puts "Hello"
    JSON.parse("{}")
    object.method_call
  end
end
"#;

        let tree = parser.parse(source, "ruby").unwrap();
        let result =
            ElementExtractor::extract_with_depth(&tree, source, "ruby", "semantic").unwrap();

        // Should find method calls
        assert!(result.calls.len() > 0, "Should find method calls");
        assert!(result.calls.iter().any(|c| c.callee_name == "puts"));
    }
}
