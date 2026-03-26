mod input_;
mod input_listen;
mod legacy_input;
mod list;
mod reedline_prompt;

use nu_protocol::{ShellError, Span};
use std::io::IsTerminal;

pub use input_::Input;
pub use input_listen::InputListen;
pub use list::InputList;

fn stdin_terminal_error(head: Span, command_name: &str) -> ShellError {
    ShellError::UnsupportedInput {
        msg: format!(
            "`{command_name}` requires an interactive terminal on stdin, but stdin is not a TTY"
        ),
        input: "stdin is not a TTY".to_string(),
        msg_span: head,
        input_span: head,
    }
}

fn output_terminal_error(head: Span, command_name: &str, stream_name: &str) -> ShellError {
    ShellError::UnsupportedInput {
        msg: format!(
            "`{command_name}` requires an interactive terminal on {stream_name}, but {stream_name} is not a TTY"
        ),
        input: format!("{stream_name} is not a TTY"),
        msg_span: head,
        input_span: head,
    }
}

pub(super) fn require_stdin_terminal(head: Span, command_name: &str) -> Result<(), ShellError> {
    if std::io::stdin().is_terminal() {
        Ok(())
    } else {
        Err(stdin_terminal_error(head, command_name))
    }
}

pub(super) fn require_stdout_terminal(head: Span, command_name: &str) -> Result<(), ShellError> {
    if std::io::stdout().is_terminal() {
        Ok(())
    } else {
        Err(output_terminal_error(head, command_name, "stdout"))
    }
}

pub(super) fn require_stderr_terminal(head: Span, command_name: &str) -> Result<(), ShellError> {
    if std::io::stderr().is_terminal() {
        Ok(())
    } else {
        Err(output_terminal_error(head, command_name, "stderr"))
    }
}
