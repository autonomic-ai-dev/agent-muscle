//! Pre-execution bash AST syntax and safety guard.
//! Intercepts syntax errors and destructive commands before execution.

use tree_sitter::Node;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardIssue {
    pub line: usize,
    pub message: String,
}

/// Validate a bash command string against AST syntax errors and destructive patterns.
/// Returns Ok(()) if the command is safe to execute, or Err(String) with the violation details.
pub fn validate_bash(command: &str) -> Result<(), String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let mut parser = tree_sitter::Parser::new();
    if parser
        .set_language(&tree_sitter_bash::LANGUAGE.into())
        .is_err()
    {
        // If parser engine fails to initialize, do not block execution
        return Ok(());
    }

    let lower_cmd = command.to_lowercase();
    if lower_cmd.contains(":(){ :|:& };:")
        || lower_cmd.contains(":(){:|:&};:")
        || lower_cmd.contains(":() { :|:& };:")
        || lower_cmd.contains(":() { :|:&; };:")
    {
        return Err("Command blocked by AST safety guard: Fork bomb pattern detected".into());
    }

    let tree = match parser.parse(command, None) {
        Some(t) => t,
        None => return Err(format!("Failed to parse command as bash: `{command}`")),
    };

    let root = tree.root_node();
    if root.has_error() {
        return Err(format!(
            "Bash AST syntax error detected in command: `{command}`"
        ));
    }

    let mut issues = Vec::new();
    walk_node(command, root, &mut issues);

    if let Some(first) = issues.into_iter().next() {
        return Err(format!(
            "Command blocked by AST safety guard (line {}): {}",
            first.line, first.message
        ));
    }

    Ok(())
}

fn walk_node(source: &str, node: Node, issues: &mut Vec<GuardIssue>) {
    let kind = node.kind();
    let line = node.start_position().row + 1;
    let text = &source[node.byte_range()];

    if kind == "command" {
        let lower = text.to_lowercase();
        let tokens: Vec<&str> = lower.split_whitespace().collect();

        // 1. Destructive recursive removal of root, home, or critical system dirs
        if !tokens.is_empty() && tokens[0] == "rm" {
            let has_recursive = tokens.iter().any(|&t| {
                t == "-rf"
                    || t == "-fr"
                    || t == "-r"
                    || t == "-f"
                    || (t.starts_with('-') && t.contains('r') && t.contains('f'))
            });

            if has_recursive {
                for token in tokens.iter().skip(1) {
                    if *token == "/"
                        || *token == "/*"
                        || *token == "~"
                        || *token == "$home"
                        || *token == "${home}"
                        || *token == "$HOME"
                        || *token == "${HOME}"
                        || *token == "/root"
                        || *token == "/etc"
                        || *token == "/usr"
                        || *token == "/var"
                        || *token == "/bin"
                        || *token == "/sbin"
                    {
                        issues.push(GuardIssue {
                            line,
                            message: format!(
                                "Destructive command blocked: rm with target `{token}`"
                            ),
                        });
                    }
                }
            }
        }

        // 2. Fork bombs (:(){ :|:& };:)
        if lower.contains(":(){ :|:& };:") || lower.contains(":(){:|:&};:") {
            issues.push(GuardIssue {
                line,
                message: "Fork bomb pattern blocked".into(),
            });
        }

        // 3. Raw block device overwriting (e.g. dd if=/dev/zero of=/dev/sd...)
        if !tokens.is_empty() && tokens[0] == "dd" {
            for token in tokens.iter().skip(1) {
                if token.starts_with("of=/dev/sd")
                    || token.starts_with("of=/dev/nvme")
                    || token.starts_with("of=/dev/disk")
                    || token.starts_with("of=/dev/hd")
                {
                    issues.push(GuardIssue {
                        line,
                        message: format!("Raw device write blocked: `{token}`"),
                    });
                }
            }
        }

        // 4. Filesystem formatting (mkfs)
        if !tokens.is_empty() && (tokens[0] == "mkfs" || tokens[0].starts_with("mkfs.")) {
            issues.push(GuardIssue {
                line,
                message: format!("Filesystem format command blocked: `{}`", tokens[0]),
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(source, child, issues);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_safe_commands() {
        assert!(validate_bash("cargo test --all-features").is_ok());
        assert!(validate_bash("git status -s").is_ok());
        assert!(validate_bash("rm -rf target/").is_ok());
        assert!(validate_bash("rm -rf /tmp/my_test_dir").is_ok());
        assert!(validate_bash("echo 'hello world'").is_ok());
    }

    #[test]
    fn detects_ast_syntax_errors() {
        // Unclosed quotes syntax error
        assert!(validate_bash("echo \"unclosed quote").is_err());
        // Incomplete bash statement
        assert!(validate_bash("if true then").is_err());
    }

    #[test]
    fn blocks_destructive_root_or_home_removal() {
        assert!(validate_bash("rm -rf /").is_err());
        assert!(validate_bash("rm -rf /*").is_err());
        assert!(validate_bash("rm -fr ~").is_err());
        assert!(validate_bash("rm -rf $HOME").is_err());
        assert!(validate_bash("rm -rf /etc").is_err());
    }

    #[test]
    fn blocks_fork_bomb() {
        assert!(validate_bash(":(){ :|:& };:").is_err());
    }

    #[test]
    fn blocks_raw_device_overwrite() {
        assert!(validate_bash("dd if=/dev/zero of=/dev/sda").is_err());
    }
}
