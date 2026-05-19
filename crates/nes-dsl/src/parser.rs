use crate::ast::{Expr, OperandSyntax};
use crate::DslError;

/// **Performance optimization:** Avoids `.collect::<Vec<_>>()` by using an iterator,
/// eliminating an O(N) heap allocation per line of DSL code parsed.
pub(crate) fn strip_comments(line: &str) -> String {
    let mut in_string = false;
    let mut escaped = false;
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            out.push(ch);
            continue;
        }
        if ch == ';' {
            break;
        }
        if ch == '/' && chars.peek() == Some(&'/') {
            break;
        }
        out.push(ch);
    }
    out
}

pub(crate) fn split_leading_label(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    let colon = trimmed.find(':')?;
    let label = trimmed[..colon].trim();
    if label.is_empty() || label.chars().any(char::is_whitespace) {
        return None;
    }
    let rest = &trimmed[colon + 1..];
    Some((label, rest))
}

pub(crate) fn split_head(line: &str) -> (&str, &str) {
    let trimmed = line.trim_start();
    let Some(split) = trimmed.find(char::is_whitespace) else {
        return (trimmed, "");
    };
    let head = &trimmed[..split];
    let rest = &trimmed[split..];
    (head, rest)
}

pub(crate) fn parse_const_assignment(input: &str, line_no: usize) -> Result<(&str, &str), DslError> {
    let Some(eq_index) = input.find('=') else {
        return Err(DslError::Parse {
            line: line_no,
            message: "expected `=` in `.const` assignment".to_owned(),
        });
    };
    let name = input[..eq_index].trim();
    let value = input[eq_index + 1..].trim();
    if name.is_empty() || value.is_empty() {
        return Err(DslError::Parse {
            line: line_no,
            message: "invalid `.const` assignment syntax".to_owned(),
        });
    }
    Ok((name, value))
}

pub(crate) fn normalize_symbol(symbol: &str) -> String {
    symbol.to_ascii_uppercase()
}

pub(crate) fn validate_symbol(symbol: &str) -> Result<(), String> {
    let Some(first) = symbol.chars().next() else {
        return Err("symbol cannot be empty".to_owned());
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(format!("symbol '{symbol}' must start with letter or underscore"));
    }
    if !symbol.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!("symbol '{symbol}' contains invalid characters"));
    }
    Ok(())
}

pub(crate) fn parse_expr(input: &str, line_no: usize) -> Result<Expr, DslError> {
    if input.is_empty() {
        return Err(DslError::Parse {
            line: line_no,
            message: "expected expression but found empty operand".to_owned(),
        });
    }
    if let Some(hex) = input.strip_prefix('$') {
        let value = i64::from_str_radix(hex, 16).map_err(|_| DslError::Parse {
            line: line_no,
            message: format!("invalid hex number '${hex}'"),
        })?;
        return Ok(Expr::Number(value));
    }
    if let Some(bin) = input.strip_prefix('%') {
        let value = i64::from_str_radix(bin, 2).map_err(|_| DslError::Parse {
            line: line_no,
            message: format!("invalid binary number '%{bin}'"),
        })?;
        return Ok(Expr::Number(value));
    }
    if let Ok(value) = input.parse::<i64>() {
        return Ok(Expr::Number(value));
    }

    let normalized = normalize_symbol(input);
    validate_symbol(&normalized).map_err(|message| DslError::Parse { line: line_no, message })?;
    Ok(Expr::Symbol(normalized))
}

pub(crate) fn split_csv(input: &str) -> Result<Vec<&str>, DslError> {
    let mut parts = Vec::new();
    let mut current_start = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = input.char_indices().peekable();

    while let Some((idx, ch)) = chars.next() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            continue;
        }
        if ch == ',' {
            parts.push(input[current_start..idx].trim());
            current_start = idx + 1;
        }
    }

    if in_string {
        return Err(DslError::Parse {
            line: 0,
            message: "unterminated string literal in list".to_owned(),
        });
    }

    let tail = input[current_start..].trim();
    if tail.is_empty() && !parts.is_empty() {
        return Err(DslError::Parse {
            line: 0,
            message: "trailing comma in list".to_owned(),
        });
    }
    if !tail.is_empty() {
        parts.push(tail);
    }

    Ok(parts)
}

pub(crate) fn is_quoted_string(value: &str) -> bool {
    value.starts_with('"') && value.ends_with('"') && value.len() >= 2
}

pub(crate) fn decode_string_literal(literal: &str) -> Result<Vec<u8>, String> {
    if !is_quoted_string(literal) {
        return Err("expected quoted string literal".to_owned());
    }

    let inner = &literal[1..literal.len() - 1];
    let mut bytes = Vec::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            if (ch as u32) > 0xFF {
                return Err(format!("character '{ch}' is outside byte range"));
            }
            bytes.push(ch as u8);
            continue;
        }

        let esc = chars
            .next()
            .ok_or_else(|| "dangling escape sequence".to_owned())?;
        match esc {
            'n' => bytes.push(b'\n'),
            'r' => bytes.push(b'\r'),
            't' => bytes.push(b'\t'),
            '\\' => bytes.push(b'\\'),
            '"' => bytes.push(b'"'),
            'x' => {
                let hi = chars
                    .next()
                    .ok_or_else(|| "expected two hex digits after \\x".to_owned())?;
                let lo = chars
                    .next()
                    .ok_or_else(|| "expected two hex digits after \\x".to_owned())?;

                let hi_val = hi
                    .to_digit(16)
                    .ok_or_else(|| "invalid hex escape sequence".to_owned())?;
                let lo_val = lo
                    .to_digit(16)
                    .ok_or_else(|| "invalid hex escape sequence".to_owned())?;
                bytes.push((hi_val << 4 | lo_val) as u8);
            }
            _ => return Err(format!("unsupported escape '\\{esc}'")),
        }
    }
    Ok(bytes)
}


pub(crate) fn parse_operand_syntax(operand: &str, line_no: usize) -> Result<OperandSyntax, DslError> {
    if operand.is_empty() {
        return Ok(OperandSyntax::Implied);
    }
    if operand.eq_ignore_ascii_case("A") {
        return Ok(OperandSyntax::Accumulator);
    }

    if let Some(raw) = operand.strip_prefix('#') {
        return Ok(OperandSyntax::Immediate(parse_expr(raw.trim(), line_no)?));
    }

    if let Some(inner) = operand.strip_prefix('(') {
        if let Some(inner) = inner.strip_suffix(",X)") {
            return Ok(OperandSyntax::IndirectX(parse_expr(inner.trim(), line_no)?));
        }
        if let Some(inner) = inner.strip_suffix("),Y") {
            return Ok(OperandSyntax::IndirectY(parse_expr(inner.trim(), line_no)?));
        }
        if let Some(inner) = inner.strip_suffix(')') {
            return Ok(OperandSyntax::Indirect(parse_expr(inner.trim(), line_no)?));
        }
    }

    if let Some(prefix) = operand.strip_suffix(",X") {
        return Ok(OperandSyntax::AbsoluteX(parse_expr(prefix.trim(), line_no)?));
    }

    if let Some(prefix) = operand.strip_suffix(",Y") {
        return Ok(OperandSyntax::AbsoluteY(parse_expr(prefix.trim(), line_no)?));
    }

    Ok(OperandSyntax::AbsoluteOrZeroPage(parse_expr(operand.trim(), line_no)?))
}
