#[cfg(test)]
mod ruby_tests {
    use crate::developer::analyze::graph::CallGraph;
    use crate::developer::analyze::parser::{ElementExtractor, ParserManager};
    use crate::developer::analyze::types::ReferenceType;
    use std::collections::HashSet;
    use std::path::PathBuf;

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

    #[test]
    fn test_ruby_reference_tracking() {
        let parser = ParserManager::new();
        let source = r#"
class User
  attr_accessor :name

  def initialize(name)
    @name = name
  end

  def greet
    puts "Hello, #{@name}"
  end
end

class Post
  STATUS_DRAFT = "draft"
  STATUS_PUBLISHED = "published"

  def initialize(title)
    @title = title
    @status = STATUS_DRAFT
  end

  def publish
    @status = STATUS_PUBLISHED
    notify_users(@status)
  end
end

def main
  user = User.new("Alice")
  post = Post.new("My Title")
  post.publish
end
"#;

        let tree = parser.parse(source, "ruby").unwrap();
        let result =
            ElementExtractor::extract_with_depth(&tree, source, "ruby", "semantic").unwrap();

        // Should find class definitions
        assert_eq!(result.class_count, 2);
        let class_names: HashSet<_> = result.classes.iter().map(|c| c.name.as_str()).collect();
        assert!(class_names.contains("User"));
        assert!(class_names.contains("Post"));

        // Should find method definitions
        assert!(result.function_count > 0);
        let method_names: HashSet<_> = result.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(method_names.contains("initialize"));
        assert!(method_names.contains("greet"));
        assert!(method_names.contains("publish"));

        // Should find constant definitions
        let constant_refs: Vec<_> = result
            .references
            .iter()
            .filter(|r| r.symbol == "STATUS_DRAFT" || r.symbol == "STATUS_PUBLISHED")
            .collect();
        assert!(
            !constant_refs.is_empty(),
            "Expected to find constant references"
        );

        // Should find class instantiation (User.new, Post.new)
        let instantiations: Vec<_> = result
            .references
            .iter()
            .filter(|r| r.ref_type == ReferenceType::TypeInstantiation)
            .collect();
        assert!(
            instantiations.len() >= 2,
            "Expected at least 2 class instantiations (User.new, Post.new)"
        );
        let instantiated_types: HashSet<_> =
            instantiations.iter().map(|r| r.symbol.as_str()).collect();
        assert!(instantiated_types.contains("User"));
        assert!(instantiated_types.contains("Post"));

        // Should find constant usage in method calls
        let constant_usages: Vec<_> = result
            .references
            .iter()
            .filter(|r| r.symbol == "STATUS_DRAFT" || r.symbol == "STATUS_PUBLISHED")
            .collect();
        assert!(
            !constant_usages.is_empty(),
            "Expected to find STATUS_* constant usages"
        );
    }

    #[test]
    fn test_ruby_call_chains() {
        let parser = ParserManager::new();

        // First file: defines User class
        let file1 = r#"
class User
  def initialize(name)
    @name = name
  end

  def display
    format_output(@name)
  end

  def format_output(text)
    "User: #{text}"
  end
end
"#;

        // Second file: uses User class
        let file2 = r#"
require_relative 'user'

def create_user(name)
  User.new(name)
end

def show_user(name)
  user = create_user(name)
  user.display
end
"#;

        // Parse both files
        let tree1 = parser.parse(file1, "ruby").unwrap();
        let result1 =
            ElementExtractor::extract_with_depth(&tree1, file1, "ruby", "semantic").unwrap();

        let tree2 = parser.parse(file2, "ruby").unwrap();
        let result2 =
            ElementExtractor::extract_with_depth(&tree2, file2, "ruby", "semantic").unwrap();

        // Build call graph
        let results = vec![
            (PathBuf::from("user.rb"), result1),
            (PathBuf::from("main.rb"), result2),
        ];
        let graph = CallGraph::build_from_results(&results);

        // Test: User class should have incoming references (instantiation)
        let incoming_user = graph.find_incoming_chains("User", 1);
        assert!(
            !incoming_user.is_empty(),
            "Expected incoming references to User class"
        );

        // Test: display method should have outgoing calls
        let outgoing_display = graph.find_outgoing_chains("display", 1);
        assert!(
            !outgoing_display.is_empty(),
            "Expected display to call format_output"
        );

        // Test: create_user should have outgoing chains (deeper depth)
        let outgoing_create = graph.find_outgoing_chains("create_user", 2);
        assert!(
            !outgoing_create.is_empty(),
            "Expected create_user to have call chains"
        );

        // Test: show_user calls create_user (incoming chain)
        let incoming_create = graph.find_incoming_chains("create_user", 1);
        assert!(
            !incoming_create.is_empty(),
            "Expected show_user to call create_user"
        );
    }
}
