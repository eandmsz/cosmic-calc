//! Engine module root. Re-exports the public surface that the UI and
//! tests consume and defines the top-level Engine struct.

pub mod ast;
pub mod errors;
pub mod eval;
pub mod format;
pub mod gamma;
pub mod input;
pub mod item;
pub mod parser;
pub mod tokenizer;

pub use errors::{CalcError, ERR_INDETERMINATE, ERR_OVERFLOW, ERR_UNDEFINED, ERR_UNDERFLOW};
pub use eval::AngleMode;
pub use format::DEFAULT_SIGNIFICANT_DIGITS;
pub use input::{CursorMove, InputBuffer};
pub use item::InputItem;

/// Output of a successful evaluation: the raw f64 value and the
/// formatted display string.
#[derive(Debug, Clone)]
pub struct EvalOutput {
    pub value: f64,
    pub display: String,
}

/// Engine ties the input buffer to the tokenize → parse → evaluate →
/// format pipeline. Evaluation is pure – it does not mutate the
/// buffer – so the UI can call it at its own cadence.
#[derive(Debug, Clone)]
pub struct Engine {
    pub input: InputBuffer,
    pub angle_mode: AngleMode,
    pub significant_digits: u8,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            input: InputBuffer::new(),
            angle_mode: AngleMode::Deg,
            significant_digits: DEFAULT_SIGNIFICANT_DIGITS,
        }
    }
}

impl Engine {
    /// Construct an engine with explicit rounding precision.
    pub fn new(significant_digits: u8) -> Self {
        Self {
            input: InputBuffer::new(),
            angle_mode: AngleMode::Deg,
            significant_digits,
        }
    }

    /// Run tokenize → parse → eval on the current buffer.
    pub fn evaluate(&self) -> Result<EvalOutput, CalcError> {
        let ascii = self.input.ascii_expression();
        evaluate_expression(&ascii, self.angle_mode, self.significant_digits)
    }

    /// Reset the input buffer (AllClear).
    pub fn clear(&mut self) {
        self.input.clear();
    }
}

/// Evaluate a raw ASCII expression string. The helper is used by the
/// tests directly (bypassing the input state machine).
pub fn evaluate_expression(
    expr: &str,
    mode: AngleMode,
    significant_digits: u8,
) -> Result<EvalOutput, CalcError> {
    let toks = tokenizer::tokenize(expr).map_err(|_| CalcError::Undefined)?;
    let ast = parser::parse(toks)?;
    let value = eval::eval(&ast, mode)?;
    let display = format::format_result(value, significant_digits);
    Ok(EvalOutput { value, display })
}

/// Convenience wrapper returning the formatted string directly and
/// translating errors to their display strings.
pub fn evaluate_to_string(expr: &str, mode: AngleMode, significant_digits: u8) -> String {
    match evaluate_expression(expr, mode, significant_digits) {
        Ok(out) => out.display,
        Err(e) => e.as_str().to_string(),
    }
}
