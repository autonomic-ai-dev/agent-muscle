//! Adaptive output compression and failure slicing.
//! Prevents token-bloat while retaining critical error traces and status summaries.

pub const DEFAULT_MAX_OUTPUT_LINES: usize = 60;

/// Compress command output adaptively based on success/failure exit codes.
pub fn compress_output(raw: &str, exit_code: i32, max_lines: usize) -> String {
    if raw.trim().is_empty() {
        return raw.to_string();
    }

    let lines: Vec<&str> = raw.lines().collect();
    let total = lines.len();

    if total <= max_lines {
        return raw.to_string();
    }

    if exit_code == 0 {
        compress_success(&lines, max_lines)
    } else {
        compress_failure(&lines, max_lines)
    }
}

fn compress_success(lines: &[&str], max_lines: usize) -> String {
    let head_count = (max_lines / 3).max(5);
    let tail_count = (max_lines / 2).max(10);

    if head_count + tail_count >= lines.len() {
        return lines.join("\n");
    }

    let skipped = lines.len() - head_count - tail_count;
    let mut out = Vec::with_capacity(head_count + tail_count + 1);

    out.extend_from_slice(&lines[..head_count]);
    let marker = format!("... [compressed {skipped} lines of repetitive output] ...");
    out.push(&marker);
    out.extend_from_slice(&lines[lines.len() - tail_count..]);

    out.join("\n")
}

fn compress_failure(lines: &[&str], max_lines: usize) -> String {
    let total = lines.len();
    let mut keep = vec![false; total];

    // Always keep initial 5 lines (invocation context)
    for i in 0..5.min(total) {
        keep[i] = true;
    }

    // Always keep final 15 lines (summary, test verdicts, panic location)
    for i in total.saturating_sub(15)..total {
        keep[i] = true;
    }

    let error_patterns = [
        "error:",
        "error[",
        "failed",
        "failure",
        "panic:",
        "assertion failed",
        "exception",
        "fatal:",
        "err:",
        "syntaxerror:",
        "traceback (most recent call last):",
    ];

    let mut found_error = false;
    for (i, line) in lines.iter().enumerate() {
        let lower = line.to_lowercase();
        if error_patterns.iter().any(|p| lower.contains(p)) {
            found_error = true;
            let start = i.saturating_sub(2);
            let end = (i + 5).min(total);
            for k in start..end {
                keep[k] = true;
            }
        }
    }

    if !found_error {
        // Fallback head/tail truncation if no specific signatures detected
        return compress_success(lines, max_lines);
    }

    let mut result = Vec::new();
    let mut i = 0;
    while i < total {
        if keep[i] {
            result.push(lines[i].to_string());
            i += 1;
        } else {
            let start_skip = i;
            while i < total && !keep[i] {
                i += 1;
            }
            let skipped = i - start_skip;
            if skipped > 2 {
                result.push(format!("... [compressed {skipped} non-error lines] ..."));
            } else {
                for k in start_skip..i {
                    result.push(lines[k].to_string());
                }
            }
        }
    }

    // If still oversized due to multiple dispersed errors, bound result to max_lines * 2
    if result.len() > max_lines * 2 {
        let head = max_lines / 2;
        let tail = max_lines;
        let skipped = result.len() - head - tail;
        let mut bounded = Vec::with_capacity(head + tail + 1);
        bounded.extend_from_slice(&result[..head]);
        let marker = format!("... [compressed {skipped} intermediate trace lines] ...");
        bounded.push(marker);
        bounded.extend_from_slice(&result[result.len() - tail..]);
        return bounded.join("\n");
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_output_unchanged() {
        let text = "line 1\nline 2\nline 3";
        assert_eq!(compress_output(text, 0, 10), text);
        assert_eq!(compress_output(text, 1, 10), text);
    }

    #[test]
    fn compresses_success_long_output() {
        let lines: Vec<String> = (1..=100).map(|i| format!("Compiling package_{i}")).collect();
        let raw = lines.join("\n");
        let compressed = compress_output(&raw, 0, 30);
        assert!(compressed.contains("compressed"));
        assert!(compressed.contains("Compiling package_1"));
        assert!(compressed.contains("Compiling package_100"));
        assert!(compressed.lines().count() <= 31);
    }

    #[test]
    fn slices_failure_output_preserving_errors() {
        let mut lines: Vec<String> = (1..=100).map(|i| format!("test test_{i} ... ok")).collect();
        lines[50] = "error[E0433]: cannot find symbol in module".to_string();
        lines[51] = "  --> src/lib.rs:10:5".to_string();
        lines.push("test result: FAILED. 99 passed; 1 failed".to_string());

        let raw = lines.join("\n");
        let compressed = compress_output(&raw, 101, 30);
        assert!(compressed.contains("error[E0433]"));
        assert!(compressed.contains("src/lib.rs:10:5"));
        assert!(compressed.contains("test result: FAILED"));
        assert!(compressed.contains("non-error lines"));
    }
}
