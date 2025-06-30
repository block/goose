/// Macro for conditional printing to stderr or stdout based on a boolean flag.
///
/// When `use_stderr` is true, prints to stderr using `eprintln!`.
/// When `use_stderr` is false, prints to stdout using `println!`.
///
/// # Examples
///
/// ```
/// use goose_cli::cli_println;
/// cli_println!(true, "This goes to stderr");
/// cli_println!(false, "This goes to stdout");
/// let some_flag = true;
/// cli_println!(some_flag, "Hello {}", "world");
/// ```
#[macro_export]
macro_rules! cli_println {
    ($use_stderr:expr, $($arg:tt)*) => {
        if $use_stderr {
            eprintln!($($arg)*);
        } else {
            println!($($arg)*);
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_cli_println_compiles() {
        // Test that the macro compiles with different argument patterns
        cli_println!(true, "test");
        cli_println!(false, "test {}", "arg");
        cli_println!(true, "test {} {}", "arg1", "arg2");

        // Test with variables
        let use_stderr = true;
        cli_println!(use_stderr, "variable test");

        let use_stderr = false;
        cli_println!(use_stderr, "variable test {}", "with arg");
    }
}
