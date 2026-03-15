//! 6502-focused DSL and assembler for generating NES ROMs.
//!
//! The DSL is intentionally assembly-first. It keeps 6502 semantics visible while
//! removing hand-encoded byte work.
//!
//! Supported directives:
//!
//! - `.org <addr>`
//! - `.bank <0|1>` (`0 => $8000`, `1 => $C000`)
//! - `.const NAME = <expr>`
//! - `.byte <expr|string>, ...`
//! - `.word <expr>, ...`
//! - `.text "literal"`
//! - `.reset <expr>`
//! - `.nmi <expr>`
//! - `.irq <expr>`
//!
//! Labels are `name:` and can be referenced from instructions/directives.

mod opcode;
use opcode::opcode_for;

use std::collections::BTreeMap;
use std::fmt;

const PRG_BANK_BYTES: usize = 16 * 1024;
const CHR_BANK_BYTES: usize = 8 * 1024;
const VEC_NMI: u16 = 0xFFFA;
const VEC_RESET: u16 = 0xFFFC;
const VEC_IRQ: u16 = 0xFFFE;

/// Nametable mirroring mode for iNES output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirroring {
    /// Horizontal nametable mirroring.
    Horizontal,
    /// Vertical nametable mirroring.
    Vertical,
}

/// Assembly configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssembleConfig {
    /// Initial program counter before any `.org`.
    pub default_org: u16,
}

impl Default for AssembleConfig {
    fn default() -> Self {
        Self {
            default_org: 0xC000,
        }
    }
}

/// ROM emission options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomBuildOptions {
    /// Cartridge nametable mirroring.
    pub mirroring: Mirroring,
    /// CHR payload. Empty means CHR RAM (`0` CHR banks in header).
    pub chr_rom: Vec<u8>,
}

impl Default for RomBuildOptions {
    fn default() -> Self {
        Self {
            mirroring: Mirroring::Horizontal,
            chr_rom: vec![0_u8; CHR_BANK_BYTES],
        }
    }
}

/// Result of assembling DSL source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    /// Initial origin used before directives changed it.
    pub default_org: u16,
    /// Output bytes keyed by absolute CPU address.
    pub bytes: BTreeMap<u16, u8>,
    /// Resolved labels (canonicalized to uppercase).
    pub labels: BTreeMap<String, u16>,
    /// NMI vector target.
    pub nmi_vector: u16,
    /// Reset vector target.
    pub reset_vector: u16,
    /// IRQ/BRK vector target.
    pub irq_vector: u16,
}

/// DSL parsing/assembly/ROM emission errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DslError {
    /// Source parse failure at a specific line.
    Parse {
        /// Line number of the error.
        line: usize,
        /// Detail message describing the parsing issue.
        message: String,
    },
    /// Duplicate label declaration.
    DuplicateLabel(String),
    /// Duplicate constant declaration.
    DuplicateConst(String),
    /// Address was written multiple times with incompatible bytes.
    DuplicateAddress {
        /// Target CPU address.
        addr: u16,
        /// Previously assembled byte value at this address.
        existing: u8,
        /// Incoming byte value trying to overwrite it.
        incoming: u8,
    },
    /// Symbol was referenced but never defined.
    UnknownSymbol(String),
    /// Value does not fit target width.
    ValueOutOfRange {
        /// Line number where the value definition failed.
        line: usize,
        /// Provided numeric value.
        value: i64,
        /// Bit-width constraint that was violated.
        width_bits: u8,
    },
    /// Relative branch cannot reach target.
    BranchOutOfRange {
        /// Line number containing the branch instruction.
        line: usize,
        /// Program counter executing the branch.
        from: u16,
        /// Target address.
        to: u16,
    },
    /// Instruction mnemonic not recognized.
    UnknownMnemonic {
        /// Line number with the unknown mnemonic.
        line: usize,
        /// The unrecognized mnemonic string.
        mnemonic: String,
    },
    /// Mnemonic exists but addressing mode is unsupported.
    UnsupportedAddressing {
        /// Line number with the unsupported syntax.
        line: usize,
        /// The mnemonic.
        mnemonic: String,
        /// The addressing mode.
        mode: String,
    },
    /// ROM layout cannot represent assembled bytes.
    InvalidRomLayout(String),
    /// Reset vector was not provided and no `RESET:` label was found.
    MissingResetVector,
}

impl fmt::Display for DslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { line, message } => write!(f, "line {line}: {message}"),
            Self::DuplicateLabel(name) => write!(f, "duplicate label '{name}'"),
            Self::DuplicateConst(name) => write!(f, "duplicate constant '{name}'"),
            Self::DuplicateAddress {
                addr,
                existing,
                incoming,
            } => write!(
                f,
                "address ${addr:04X} already written with {existing:02X}, tried to write {incoming:02X}"
            ),
            Self::UnknownSymbol(name) => write!(f, "unknown symbol '{name}'"),
            Self::ValueOutOfRange {
                line,
                value,
                width_bits,
            } => write!(
                f,
                "line {line}: value {value} does not fit in {width_bits}-bit field"
            ),
            Self::BranchOutOfRange { line, from, to } => write!(
                f,
                "line {line}: branch from ${from:04X} to ${to:04X} out of range"
            ),
            Self::UnknownMnemonic { line, mnemonic } => {
                write!(f, "line {line}: unknown mnemonic '{mnemonic}'")
            }
            Self::UnsupportedAddressing {
                line,
                mnemonic,
                mode,
            } => write!(
                f,
                "line {line}: addressing mode {mode} not supported for {mnemonic}"
            ),
            Self::InvalidRomLayout(message) => f.write_str(message),
            Self::MissingResetVector => {
                f.write_str("missing reset vector (use `.reset` or `reset:` label)")
            }
        }
    }
}

impl std::error::Error for DslError {}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Expr {
    Number(i64),
    Symbol(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OperandSyntax {
    Implied,
    Accumulator,
    Immediate(Expr),
    IndirectX(Expr),
    IndirectY(Expr),
    Indirect(Expr),
    AbsoluteX(Expr),
    AbsoluteY(Expr),
    AbsoluteOrZeroPage(Expr),
}

fn parse_operand_syntax(operand: &str, line_no: usize) -> Result<OperandSyntax, DslError> {
    if operand.is_empty() {
        return Ok(OperandSyntax::Implied);
    }
    if operand.eq_ignore_ascii_case("A") {
        return Ok(OperandSyntax::Accumulator);
    }
    if let Some(raw) = operand.strip_prefix('#') {
        return Ok(OperandSyntax::Immediate(parse_expr(raw.trim(), line_no)?));
    }
    if let Some(inner) = operand
        .strip_prefix('(')
        .and_then(|v| v.strip_suffix(",X)"))
    {
        return Ok(OperandSyntax::IndirectX(parse_expr(inner.trim(), line_no)?));
    }
    if let Some(inner) = operand
        .strip_prefix('(')
        .and_then(|v| v.strip_suffix("),Y"))
    {
        return Ok(OperandSyntax::IndirectY(parse_expr(inner.trim(), line_no)?));
    }
    if let Some(inner) = operand.strip_prefix('(').and_then(|v| v.strip_suffix(')')) {
        return Ok(OperandSyntax::Indirect(parse_expr(inner.trim(), line_no)?));
    }
    if let Some(prefix) = operand.strip_suffix(",X") {
        return Ok(OperandSyntax::AbsoluteX(parse_expr(
            prefix.trim(),
            line_no,
        )?));
    }
    if let Some(prefix) = operand.strip_suffix(",Y") {
        return Ok(OperandSyntax::AbsoluteY(parse_expr(
            prefix.trim(),
            line_no,
        )?));
    }
    Ok(OperandSyntax::AbsoluteOrZeroPage(parse_expr(
        operand.trim(),
        line_no,
    )?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddressingMode {
    Implied,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
    Relative,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FixupKind {
    Byte,
    Word,
    Relative,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Fixup {
    line: usize,
    addr: u16,
    expr: Expr,
    kind: FixupKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct VectorExprs {
    nmi: Option<Expr>,
    reset: Option<Expr>,
    irq: Option<Expr>,
}

#[derive(Debug, Clone)]
struct Assembler {
    config: AssembleConfig,
    current_addr: u16,
    bytes: BTreeMap<u16, u8>,
    labels: BTreeMap<String, u16>,
    constants: BTreeMap<String, i64>,
    fixups: Vec<Fixup>,
    vectors: VectorExprs,
}

impl Assembler {
    fn new(config: AssembleConfig) -> Self {
        Self {
            config,
            current_addr: config.default_org,
            bytes: BTreeMap::new(),
            labels: BTreeMap::new(),
            constants: BTreeMap::new(),
            fixups: Vec::new(),
            vectors: VectorExprs::default(),
        }
    }

    fn assemble_line(&mut self, line_no: usize, raw_line: &str) -> Result<(), DslError> {
        let stripped = strip_comments(raw_line);
        let mut line = stripped.trim();
        if line.is_empty() || line.starts_with('#') {
            return Ok(());
        }

        while let Some((label, rest)) = split_leading_label(line) {
            self.define_label(label)?;
            line = rest.trim_start();
            if line.is_empty() {
                return Ok(());
            }
        }

        if line.starts_with('.') {
            self.handle_directive(line_no, line)
        } else {
            self.handle_instruction(line_no, line)
        }
    }

    fn finalize(mut self) -> Result<AssembledProgram, DslError> {
        let fixups = std::mem::take(&mut self.fixups);
        for fixup in fixups {
            let value = self.resolve_expr(&fixup.expr)?;
            match fixup.kind {
                FixupKind::Byte => {
                    let byte = fit_u8(value, fixup.line)?;
                    self.bytes.insert(fixup.addr, byte);
                }
                FixupKind::Word => {
                    let word = fit_u16(value, fixup.line)?;
                    let [lo, hi] = word.to_le_bytes();
                    self.bytes.insert(fixup.addr, lo);
                    self.bytes.insert(fixup.addr.wrapping_add(1), hi);
                }
                FixupKind::Relative => {
                    let target = fit_u16(value, fixup.line)?;
                    let next_pc = fixup.addr.wrapping_add(1);
                    let delta = i32::from(target) - i32::from(next_pc);
                    if !(-128..=127).contains(&delta) {
                        return Err(DslError::BranchOutOfRange {
                            line: fixup.line,
                            from: next_pc,
                            to: target,
                        });
                    }
                    self.bytes.insert(fixup.addr, (delta as i8) as u8);
                }
            }
        }

        let reset_vector = self.resolve_vector(self.vectors.reset.as_ref(), "RESET", true)?;
        let nmi_vector = self.resolve_vector(self.vectors.nmi.as_ref(), "NMI", false)?;
        let irq_vector = self.resolve_vector(self.vectors.irq.as_ref(), "IRQ", false)?;

        Ok(AssembledProgram {
            default_org: self.config.default_org,
            bytes: self.bytes,
            labels: self.labels,
            nmi_vector,
            reset_vector,
            irq_vector,
        })
    }

    fn resolve_vector(
        &self,
        expr: Option<&Expr>,
        default_label: &str,
        required: bool,
    ) -> Result<u16, DslError> {
        if let Some(expr) = expr {
            return fit_u16(self.resolve_expr(expr)?, 0);
        }
        if let Some(addr) = self.labels.get(default_label) {
            return Ok(*addr);
        }
        if required {
            Err(DslError::MissingResetVector)
        } else {
            Ok(self.config.default_org)
        }
    }

    fn resolve_expr(&self, expr: &Expr) -> Result<i64, DslError> {
        match expr {
            Expr::Number(value) => Ok(*value),
            Expr::Symbol(name) => self
                .resolve_symbol(name)
                .ok_or_else(|| DslError::UnknownSymbol(name.clone())),
        }
    }

    fn resolve_symbol(&self, symbol: &str) -> Option<i64> {
        self.constants
            .get(symbol)
            .copied()
            .or_else(|| self.labels.get(symbol).copied().map(i64::from))
    }
    fn define_label(&mut self, name: &str) -> Result<(), DslError> {
        let normalized = normalize_symbol(name);
        validate_symbol(&normalized).map_err(|message| DslError::Parse { line: 0, message })?;
        if self.constants.contains_key(&normalized) {
            return Err(DslError::DuplicateLabel(normalized));
        }
        if self
            .labels
            .insert(normalized.clone(), self.current_addr)
            .is_some()
        {
            return Err(DslError::DuplicateLabel(normalized));
        }
        Ok(())
    }

    fn handle_directive(&mut self, line_no: usize, line: &str) -> Result<(), DslError> {
        let (name, rest) = split_head(line);
        let directive = name.to_ascii_lowercase();
        let args = rest.trim();
        match directive.as_str() {
            ".org" => {
                let expr = parse_expr(args, line_no)?;
                let value = fit_u16(self.resolve_expr(&expr)?, line_no)?;
                self.current_addr = value;
                Ok(())
            }
            ".bank" => {
                let expr = parse_expr(args, line_no)?;
                let bank = self.resolve_expr(&expr)?;
                self.current_addr = match bank {
                    0 => 0x8000,
                    1 => 0xC000,
                    _ => {
                        return Err(DslError::Parse {
                            line: line_no,
                            message: "only `.bank 0` and `.bank 1` are supported".to_owned(),
                        });
                    }
                };
                Ok(())
            }
            ".const" => {
                let (name, value_expr) = parse_const_assignment(args, line_no)?;
                let normalized = normalize_symbol(name);
                validate_symbol(&normalized).map_err(|message| DslError::Parse {
                    line: line_no,
                    message,
                })?;
                if self.labels.contains_key(&normalized) || self.constants.contains_key(&normalized)
                {
                    return Err(DslError::DuplicateConst(normalized));
                }
                let value = self.resolve_expr(&parse_expr(value_expr, line_no)?)?;
                self.constants.insert(normalized, value);
                Ok(())
            }
            ".byte" => {
                for arg in split_csv(args)? {
                    if is_quoted_string(&arg) {
                        let bytes =
                            decode_string_literal(&arg).map_err(|message| DslError::Parse {
                                line: line_no,
                                message,
                            })?;
                        for byte in bytes {
                            self.emit_u8(byte)?;
                        }
                    } else {
                        self.emit_expr_byte(parse_expr(&arg, line_no)?, line_no, FixupKind::Byte)?;
                    }
                }
                Ok(())
            }
            ".word" => {
                for arg in split_csv(args)? {
                    self.emit_expr_word(parse_expr(&arg, line_no)?, line_no)?;
                }
                Ok(())
            }
            ".text" => {
                let literals = split_csv(args)?;
                if literals.is_empty() {
                    return Err(DslError::Parse {
                        line: line_no,
                        message: ".text expects at least one quoted string".to_owned(),
                    });
                }
                for lit in literals {
                    if !is_quoted_string(&lit) {
                        return Err(DslError::Parse {
                            line: line_no,
                            message: ".text accepts only quoted string literals".to_owned(),
                        });
                    }
                    let bytes = decode_string_literal(&lit).map_err(|message| DslError::Parse {
                        line: line_no,
                        message,
                    })?;
                    for byte in bytes {
                        self.emit_u8(byte)?;
                    }
                }
                Ok(())
            }
            ".reset" => {
                self.vectors.reset = Some(parse_expr(args, line_no)?);
                Ok(())
            }
            ".nmi" => {
                self.vectors.nmi = Some(parse_expr(args, line_no)?);
                Ok(())
            }
            ".irq" => {
                self.vectors.irq = Some(parse_expr(args, line_no)?);
                Ok(())
            }
            _ => Err(DslError::Parse {
                line: line_no,
                message: format!("unknown directive '{directive}'"),
            }),
        }
    }

    fn handle_instruction(&mut self, line_no: usize, line: &str) -> Result<(), DslError> {
        let (head, rest) = split_head(line);
        let mnemonic = head.to_ascii_uppercase();
        let operand = rest.trim();

        if is_branch_mnemonic(&mnemonic) {
            if operand.is_empty() {
                return Err(DslError::Parse {
                    line: line_no,
                    message: format!("{mnemonic} expects a target operand"),
                });
            }
            let opcode = opcode_for(&mnemonic, AddressingMode::Relative).ok_or_else(|| {
                DslError::UnsupportedAddressing {
                    line: line_no,
                    mnemonic: mnemonic.clone(),
                    mode: "relative".to_owned(),
                }
            })?;
            self.emit_u8(opcode)?;
            self.emit_expr_byte(parse_expr(operand, line_no)?, line_no, FixupKind::Relative)?;
            return Ok(());
        }

        let syntax = parse_operand_syntax(operand, line_no)?;

        let (mode, kind, expr_opt) = match syntax {
            OperandSyntax::Implied => (AddressingMode::Implied, None, None),
            OperandSyntax::Accumulator => (AddressingMode::Accumulator, None, None),
            OperandSyntax::Immediate(expr) => {
                (AddressingMode::Immediate, Some(FixupKind::Byte), Some(expr))
            }
            OperandSyntax::IndirectX(expr) => {
                (AddressingMode::IndirectX, Some(FixupKind::Byte), Some(expr))
            }
            OperandSyntax::IndirectY(expr) => {
                (AddressingMode::IndirectY, Some(FixupKind::Byte), Some(expr))
            }
            OperandSyntax::Indirect(expr) => {
                (AddressingMode::Indirect, Some(FixupKind::Word), Some(expr))
            }
            OperandSyntax::AbsoluteX(expr) => {
                if self.can_use_zeropage(&expr)
                    && opcode_for(&mnemonic, AddressingMode::ZeroPageX).is_some()
                {
                    (AddressingMode::ZeroPageX, Some(FixupKind::Byte), Some(expr))
                } else {
                    (AddressingMode::AbsoluteX, Some(FixupKind::Word), Some(expr))
                }
            }
            OperandSyntax::AbsoluteY(expr) => {
                if self.can_use_zeropage(&expr)
                    && opcode_for(&mnemonic, AddressingMode::ZeroPageY).is_some()
                {
                    (AddressingMode::ZeroPageY, Some(FixupKind::Byte), Some(expr))
                } else {
                    (AddressingMode::AbsoluteY, Some(FixupKind::Word), Some(expr))
                }
            }
            OperandSyntax::AbsoluteOrZeroPage(expr) => {
                if self.can_use_zeropage(&expr)
                    && opcode_for(&mnemonic, AddressingMode::ZeroPage).is_some()
                {
                    (AddressingMode::ZeroPage, Some(FixupKind::Byte), Some(expr))
                } else {
                    (AddressingMode::Absolute, Some(FixupKind::Word), Some(expr))
                }
            }
        };

        let opcode = opcode_for(&mnemonic, mode)
            .ok_or_else(|| unknown_or_mode_error(line_no, &mnemonic, mode))?;
        self.emit_u8(opcode)?;

        if let (Some(kind), Some(expr)) = (kind, expr_opt) {
            match kind {
                FixupKind::Byte => self.emit_expr_byte(expr, line_no, FixupKind::Byte)?,
                FixupKind::Word => self.emit_expr_word(expr, line_no)?,
                FixupKind::Relative => unreachable!("relative mode handled earlier"),
            }
        }

        Ok(())
    }

    fn can_use_zeropage(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Number(value) => (0..=255).contains(value),
            Expr::Symbol(symbol) => self
                .resolve_symbol(symbol)
                .is_some_and(|value| (0..=255).contains(&value)),
        }
    }

    fn emit_expr_word(&mut self, expr: Expr, line_no: usize) -> Result<(), DslError> {
        match expr {
            Expr::Number(value) => {
                let value = fit_u16(value, line_no)?;
                let [lo, hi] = value.to_le_bytes();
                self.emit_u8(lo)?;
                self.emit_u8(hi)
            }
            Expr::Symbol(symbol) => {
                if let Some(value) = self.resolve_symbol(&symbol) {
                    let value = fit_u16(value, line_no)?;
                    let [lo, hi] = value.to_le_bytes();
                    self.emit_u8(lo)?;
                    self.emit_u8(hi)?;
                } else {
                    let fixup_addr = self.current_addr;
                    self.emit_u8(0)?;
                    self.emit_u8(0)?;
                    self.fixups.push(Fixup {
                        line: line_no,
                        addr: fixup_addr,
                        expr: Expr::Symbol(symbol),
                        kind: FixupKind::Word,
                    });
                }
                Ok(())
            }
        }
    }

    fn emit_expr_byte(
        &mut self,
        expr: Expr,
        line_no: usize,
        kind: FixupKind,
    ) -> Result<(), DslError> {
        match (&expr, &kind) {
            (Expr::Number(value), FixupKind::Byte) => self.emit_u8(fit_u8(*value, line_no)?),
            (Expr::Number(value), FixupKind::Relative) => {
                let target = fit_u16(*value, line_no)?;
                let next_pc = self.current_addr.wrapping_add(1);
                let delta = i32::from(target) - i32::from(next_pc);
                if !(-128..=127).contains(&delta) {
                    return Err(DslError::BranchOutOfRange {
                        line: line_no,
                        from: next_pc,
                        to: target,
                    });
                }
                self.emit_u8((delta as i8) as u8)
            }
            (Expr::Symbol(symbol), FixupKind::Byte) => {
                if let Some(value) = self.resolve_symbol(symbol) {
                    self.emit_u8(fit_u8(value, line_no)?)
                } else {
                    let fixup_addr = self.current_addr;
                    self.emit_u8(0)?;
                    self.fixups.push(Fixup {
                        line: line_no,
                        addr: fixup_addr,
                        expr,
                        kind: FixupKind::Byte,
                    });
                    Ok(())
                }
            }
            (Expr::Symbol(symbol), FixupKind::Relative) => {
                if let Some(value) = self.resolve_symbol(symbol) {
                    let target = fit_u16(value, line_no)?;
                    let next_pc = self.current_addr.wrapping_add(1);
                    let delta = i32::from(target) - i32::from(next_pc);
                    if !(-128..=127).contains(&delta) {
                        return Err(DslError::BranchOutOfRange {
                            line: line_no,
                            from: next_pc,
                            to: target,
                        });
                    }
                    self.emit_u8((delta as i8) as u8)
                } else {
                    let fixup_addr = self.current_addr;
                    self.emit_u8(0)?;
                    self.fixups.push(Fixup {
                        line: line_no,
                        addr: fixup_addr,
                        expr,
                        kind: FixupKind::Relative,
                    });
                    Ok(())
                }
            }
            (_, FixupKind::Word) => Err(DslError::Parse {
                line: line_no,
                message: "internal error: byte emitter used for word fixup".to_owned(),
            }),
        }
    }

    fn emit_u8(&mut self, byte: u8) -> Result<(), DslError> {
        self.write_resolved_byte(self.current_addr, byte)?;
        self.current_addr = self.current_addr.wrapping_add(1);
        Ok(())
    }

    fn write_resolved_byte(&mut self, addr: u16, byte: u8) -> Result<(), DslError> {
        if let Some(existing) = self.bytes.get(&addr) {
            if *existing != byte {
                return Err(DslError::DuplicateAddress {
                    addr,
                    existing: *existing,
                    incoming: byte,
                });
            }
            return Ok(());
        }
        self.bytes.insert(addr, byte);
        Ok(())
    }
}

/// Assembles source with default configuration.
///
/// # Errors
///
/// Returns [`DslError`] for parse, symbol, or encoding failures.
pub fn assemble(source: &str) -> Result<AssembledProgram, DslError> {
    assemble_with_config(source, AssembleConfig::default())
}

/// Assembles source with explicit configuration.
///
/// # Errors
///
/// Returns [`DslError`] for parse, symbol, or encoding failures.
pub fn assemble_with_config(
    source: &str,
    config: AssembleConfig,
) -> Result<AssembledProgram, DslError> {
    let mut assembler = Assembler::new(config);
    for (idx, raw_line) in source.lines().enumerate() {
        assembler.assemble_line(idx + 1, raw_line)?;
    }
    assembler.finalize()
}

/// Builds an iNES mapper-0 ROM from DSL source.
///
/// # Errors
///
/// Returns [`DslError`] if assembly fails or ROM layout is invalid.
pub fn build_ines_nrom_rom(source: &str, options: &RomBuildOptions) -> Result<Vec<u8>, DslError> {
    let program = assemble(source)?;
    emit_ines_nrom_rom(&program, options)
}

/// Emits an iNES mapper-0 ROM from an assembled program.
///
/// # Errors
///
/// Returns [`DslError`] when output cannot fit NROM layout constraints.
pub fn emit_ines_nrom_rom(
    program: &AssembledProgram,
    options: &RomBuildOptions,
) -> Result<Vec<u8>, DslError> {
    let uses_lower_bank = program.bytes.keys().any(|addr| *addr < 0xC000);
    let prg_banks = if uses_lower_bank { 2_usize } else { 1_usize };
    let prg_len = prg_banks * PRG_BANK_BYTES;

    let mut prg = vec![0_u8; prg_len];
    let mut prg_written = vec![false; prg_len];

    for (addr, byte) in &program.bytes {
        insert_mapped_byte(
            &mut prg,
            &mut prg_written,
            prg_banks,
            *addr,
            *byte,
            "program bytes",
        )?;
    }

    write_vector(
        &mut prg,
        &mut prg_written,
        prg_banks,
        VEC_NMI,
        program.nmi_vector,
        "nmi",
    )?;
    write_vector(
        &mut prg,
        &mut prg_written,
        prg_banks,
        VEC_RESET,
        program.reset_vector,
        "reset",
    )?;
    write_vector(
        &mut prg,
        &mut prg_written,
        prg_banks,
        VEC_IRQ,
        program.irq_vector,
        "irq",
    )?;

    let chr_banks = if options.chr_rom.is_empty() {
        0_usize
    } else {
        options.chr_rom.len().div_ceil(CHR_BANK_BYTES)
    };
    if chr_banks > usize::from(u8::MAX) {
        return Err(DslError::InvalidRomLayout(format!(
            "CHR bank count {} exceeds iNES header limit",
            chr_banks
        )));
    }

    let mut rom = Vec::with_capacity(16 + prg_len + chr_banks * CHR_BANK_BYTES);
    rom.extend_from_slice(b"NES\x1A");
    rom.push(prg_banks as u8);
    rom.push(chr_banks as u8);
    let mut flags6 = 0_u8;
    if options.mirroring == Mirroring::Vertical {
        flags6 |= 0x01;
    }
    rom.push(flags6);
    rom.push(0_u8);
    rom.extend_from_slice(&[0_u8; 8]);
    rom.extend_from_slice(&prg);
    if chr_banks != 0 {
        rom.extend_from_slice(&options.chr_rom);
        let pad = chr_banks * CHR_BANK_BYTES - options.chr_rom.len();
        if pad != 0 {
            rom.extend(std::iter::repeat_n(0_u8, pad));
        }
    }

    Ok(rom)
}

fn write_vector(
    prg: &mut [u8],
    written: &mut [bool],
    prg_banks: usize,
    addr: u16,
    target: u16,
    vector_name: &str,
) -> Result<(), DslError> {
    let [lo, hi] = target.to_le_bytes();
    insert_mapped_byte(
        prg,
        written,
        prg_banks,
        addr,
        lo,
        &format!("{vector_name} vector lo"),
    )?;
    insert_mapped_byte(
        prg,
        written,
        prg_banks,
        addr.wrapping_add(1),
        hi,
        &format!("{vector_name} vector hi"),
    )
}

fn insert_mapped_byte(
    prg: &mut [u8],
    written: &mut [bool],
    prg_banks: usize,
    addr: u16,
    value: u8,
    context: &str,
) -> Result<(), DslError> {
    let Some(offset) = cpu_addr_to_prg_offset(prg_banks, addr) else {
        return Err(DslError::InvalidRomLayout(format!(
            "{context}: CPU address ${addr:04X} is outside PRG ROM window ($8000-$FFFF)"
        )));
    };
    if offset >= prg.len() {
        return Err(DslError::InvalidRomLayout(format!(
            "{context}: mapped offset {offset} exceeds PRG image length {}",
            prg.len()
        )));
    }
    if written[offset] && prg[offset] != value {
        return Err(DslError::InvalidRomLayout(format!(
            "{context}: conflicting writes at CPU ${addr:04X} (existing {:02X}, incoming {value:02X})",
            prg[offset]
        )));
    }
    prg[offset] = value;
    written[offset] = true;
    Ok(())
}

fn cpu_addr_to_prg_offset(prg_banks: usize, addr: u16) -> Option<usize> {
    if addr < 0x8000 {
        return None;
    }
    let raw = usize::from(addr - 0x8000);
    match prg_banks {
        1 => Some(raw % PRG_BANK_BYTES),
        2 => Some(raw),
        _ => None,
    }
}

fn unknown_or_mode_error(line: usize, mnemonic: &str, mode: AddressingMode) -> DslError {
    if has_mnemonic(mnemonic) {
        DslError::UnsupportedAddressing {
            line,
            mnemonic: mnemonic.to_owned(),
            mode: mode_name(mode).to_owned(),
        }
    } else {
        DslError::UnknownMnemonic {
            line,
            mnemonic: mnemonic.to_owned(),
        }
    }
}

fn has_mnemonic(mnemonic: &str) -> bool {
    matches!(
        mnemonic,
        "ADC"
            | "AND"
            | "ASL"
            | "BCC"
            | "BCS"
            | "BEQ"
            | "BIT"
            | "BMI"
            | "BNE"
            | "BPL"
            | "BRK"
            | "BVC"
            | "BVS"
            | "CLC"
            | "CLD"
            | "CLI"
            | "CLV"
            | "CMP"
            | "CPX"
            | "CPY"
            | "DEC"
            | "DEX"
            | "DEY"
            | "EOR"
            | "INC"
            | "INX"
            | "INY"
            | "JMP"
            | "JSR"
            | "LDA"
            | "LDX"
            | "LDY"
            | "LSR"
            | "NOP"
            | "ORA"
            | "PHA"
            | "PHP"
            | "PLA"
            | "PLP"
            | "ROL"
            | "ROR"
            | "RTI"
            | "RTS"
            | "SBC"
            | "SEC"
            | "SED"
            | "SEI"
            | "STA"
            | "STX"
            | "STY"
            | "TAX"
            | "TAY"
            | "TSX"
            | "TXA"
            | "TXS"
            | "TYA"
    )
}

fn mode_name(mode: AddressingMode) -> &'static str {
    match mode {
        AddressingMode::Implied => "implied",
        AddressingMode::Accumulator => "accumulator",
        AddressingMode::Immediate => "immediate",
        AddressingMode::ZeroPage => "zeropage",
        AddressingMode::ZeroPageX => "zeropage,X",
        AddressingMode::ZeroPageY => "zeropage,Y",
        AddressingMode::Absolute => "absolute",
        AddressingMode::AbsoluteX => "absolute,X",
        AddressingMode::AbsoluteY => "absolute,Y",
        AddressingMode::Indirect => "indirect",
        AddressingMode::IndirectX => "(indirect,X)",
        AddressingMode::IndirectY => "(indirect),Y",
        AddressingMode::Relative => "relative",
    }
}

fn is_branch_mnemonic(mnemonic: &str) -> bool {
    matches!(
        mnemonic,
        "BCC" | "BCS" | "BEQ" | "BMI" | "BNE" | "BPL" | "BVC" | "BVS"
    )
}

fn fit_u8(value: i64, line: usize) -> Result<u8, DslError> {
    u8::try_from(value).map_err(|_| DslError::ValueOutOfRange {
        line,
        value,
        width_bits: 8,
    })
}

fn fit_u16(value: i64, line: usize) -> Result<u16, DslError> {
    u16::try_from(value).map_err(|_| DslError::ValueOutOfRange {
        line,
        value,
        width_bits: 16,
    })
}

fn strip_comments(line: &str) -> String {
    let mut in_string = false;
    let mut escaped = false;
    let mut out = String::with_capacity(line.len());
    let chars: Vec<char> = line.chars().collect();
    let mut idx = 0usize;
    while idx < chars.len() {
        let ch = chars[idx];
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            idx += 1;
            continue;
        }
        if ch == '"' {
            in_string = true;
            out.push(ch);
            idx += 1;
            continue;
        }
        if ch == ';' {
            break;
        }
        if ch == '/' && idx + 1 < chars.len() && chars[idx + 1] == '/' {
            break;
        }
        out.push(ch);
        idx += 1;
    }
    out
}

fn split_leading_label(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    let colon = trimmed.find(':')?;
    let label = trimmed[..colon].trim();
    if label.is_empty() || label.chars().any(char::is_whitespace) {
        return None;
    }
    let rest = &trimmed[colon + 1..];
    Some((label, rest))
}

fn split_head(line: &str) -> (&str, &str) {
    let trimmed = line.trim_start();
    if let Some(split) = trimmed.find(char::is_whitespace) {
        let head = &trimmed[..split];
        let rest = &trimmed[split..];
        (head, rest)
    } else {
        (trimmed, "")
    }
}

fn parse_const_assignment(input: &str, line_no: usize) -> Result<(&str, &str), DslError> {
    let Some(eq_index) = input.find('=') else {
        return Err(DslError::Parse {
            line: line_no,
            message: ".const expects `NAME = value`".to_owned(),
        });
    };
    let name = input[..eq_index].trim();
    let value = input[eq_index + 1..].trim();
    if name.is_empty() || value.is_empty() {
        return Err(DslError::Parse {
            line: line_no,
            message: ".const expects `NAME = value`".to_owned(),
        });
    }
    Ok((name, value))
}

fn normalize_symbol(symbol: &str) -> String {
    symbol.trim().to_ascii_uppercase()
}

fn validate_symbol(symbol: &str) -> Result<(), String> {
    let mut chars = symbol.chars();
    let Some(first) = chars.next() else {
        return Err("symbol cannot be empty".to_owned());
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err("symbol must start with [A-Za-z_]".to_owned());
    }
    if chars.any(|ch| !(ch.is_ascii_alphanumeric() || ch == '_')) {
        return Err("symbol must contain only [A-Za-z0-9_]".to_owned());
    }
    Ok(())
}

fn parse_expr(input: &str, line_no: usize) -> Result<Expr, DslError> {
    let token = input.trim();
    if token.is_empty() {
        return Err(DslError::Parse {
            line: line_no,
            message: "expected expression".to_owned(),
        });
    }

    let (sign, rest) = if let Some(rest) = token.strip_prefix('-') {
        (-1_i64, rest.trim())
    } else if let Some(rest) = token.strip_prefix('+') {
        (1_i64, rest.trim())
    } else {
        (1_i64, token)
    };

    let parsed = if let Some(hex) = rest.strip_prefix('$') {
        i64::from_str_radix(hex, 16).ok()
    } else if let Some(bin) = rest.strip_prefix('%') {
        i64::from_str_radix(bin, 2).ok()
    } else if let Some(hex) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).ok()
    } else {
        rest.parse::<i64>().ok()
    };

    if let Some(value) = parsed {
        let signed = match sign {
            -1 => -value,
            1 => value,
            _ => unreachable!("parse_expr sign is always +/-1"),
        };
        return Ok(Expr::Number(signed));
    }

    if sign != 1 {
        return Err(DslError::Parse {
            line: line_no,
            message: format!("invalid numeric expression '{token}'"),
        });
    }

    let symbol = normalize_symbol(rest);
    validate_symbol(&symbol).map_err(|message| DslError::Parse {
        line: line_no,
        message,
    })?;
    Ok(Expr::Symbol(symbol))
}

fn split_csv(input: &str) -> Result<Vec<String>, DslError> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;

    for ch in input.chars() {
        if in_string {
            current.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                current.push(ch);
            }
            ',' => {
                let item = current.trim();
                if !item.is_empty() {
                    out.push(item.to_owned());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if in_string {
        return Err(DslError::Parse {
            line: 0,
            message: "unterminated string literal".to_owned(),
        });
    }

    let tail = current.trim();
    if !tail.is_empty() {
        out.push(tail.to_owned());
    }
    Ok(out)
}

fn is_quoted_string(value: &str) -> bool {
    value.len() >= 2 && value.starts_with('"') && value.ends_with('"')
}

fn decode_string_literal(literal: &str) -> Result<Vec<u8>, String> {
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

                // **Performance optimization:** Replaced intermediate string allocation
                // during hex escape parsing with direct character parsing.
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

#[cfg(test)]
mod tests {
    use super::*;
    use nes_core::{Command, NesCore};

    #[test]
    fn assembles_labels_branches_and_vectors() {
        let source = r#"
            .org $C000
            RESET:
                LDX #$00
            loop:
                INX
                CPX #$05
                BNE loop
                JMP RESET
        "#;

        let program = assemble(source).expect("assembly should succeed");
        assert_eq!(program.reset_vector, 0xC000);
        assert_eq!(program.nmi_vector, 0xC000);
        assert_eq!(program.irq_vector, 0xC000);
        assert_eq!(program.bytes.get(&0xC000), Some(&0xA2));
        assert_eq!(program.bytes.get(&0xC001), Some(&0x00));
        assert_eq!(program.bytes.get(&0xC005), Some(&0xD0));
        assert_eq!(program.bytes.get(&0xC006), Some(&0xFB));
        assert_eq!(program.bytes.get(&0xC007), Some(&0x4C));
    }

    #[test]
    fn supports_data_directives_and_literals() {
        let source = r#"
            .org $C100
            .byte $01, 2, "AB", $FF
            .word $1234
            .text "Z\n"
            .reset $C100
        "#;

        let program = assemble(source).expect("assembly should succeed");
        let expected = [
            (0xC100, 0x01),
            (0xC101, 0x02),
            (0xC102, b'A'),
            (0xC103, b'B'),
            (0xC104, 0xFF),
            (0xC105, 0x34),
            (0xC106, 0x12),
            (0xC107, b'Z'),
            (0xC108, b'\n'),
        ];
        for (addr, byte) in expected {
            assert_eq!(
                program.bytes.get(&addr),
                Some(&byte),
                "byte mismatch at {addr:04X}"
            );
        }
    }

    #[test]
    fn emits_two_prg_banks_when_bank0_is_used() {
        let source = r#"
            .bank 0
            LDA #$11
            STA $0002
            .bank 1
            RESET:
                JMP RESET
        "#;
        let program = assemble(source).expect("assembly should succeed");
        let rom = emit_ines_nrom_rom(&program, &RomBuildOptions::default())
            .expect("rom emit should succeed");

        assert_eq!(rom[0..4], [0x4E, 0x45, 0x53, 0x1A]);
        assert_eq!(rom[4], 2, "expected 2 PRG banks");

        let prg_start = 16;
        assert_eq!(rom[prg_start], 0xA9);
        assert_eq!(rom[prg_start + 1], 0x11);
        assert_eq!(rom[prg_start + 2], 0x85);
        assert_eq!(rom[prg_start + 3], 0x02);
    }

    #[test]
    fn emitted_rom_boots_in_nes_core() {
        let source = r#"
            .bank 1
            RESET:
                SEI
                CLD
            loop:
                JMP loop
            .reset RESET
            .nmi RESET
            .irq RESET
        "#;

        let rom = build_ines_nrom_rom(source, &RomBuildOptions::default())
            .expect("rom build should succeed");
        let mut core = NesCore::new();
        core.load_ines_rom(&rom).expect("rom should load");
        assert_eq!(core.cpu_pc(), 0xC000);

        core.execute(Command::StepCpu).expect("step 1");
        assert_eq!(core.cpu_pc(), 0xC001);

        core.execute(Command::StepCpu).expect("step 2");
        assert_eq!(core.cpu_pc(), 0xC002);

        core.execute(Command::StepCpu).expect("loop jump");
        assert_eq!(core.cpu_pc(), 0xC002);
    }

    #[test]
    fn rom_build_options_default_uses_8k_chr_bank() {
        let defaults = RomBuildOptions::default();
        assert_eq!(defaults.chr_rom.len(), 8 * 1024);
    }

    #[test]
    fn dsl_error_display_includes_context() {
        let duplicate = DslError::DuplicateLabel("LOOP".to_owned()).to_string();
        assert_eq!(duplicate, "duplicate label 'LOOP'");

        let unsupported = DslError::UnsupportedAddressing {
            line: 9,
            mnemonic: "LDA".to_owned(),
            mode: "indirect".to_owned(),
        }
        .to_string();
        assert_eq!(
            unsupported,
            "line 9: addressing mode indirect not supported for LDA"
        );

        let missing_reset = DslError::MissingResetVector.to_string();
        assert!(missing_reset.contains("missing reset vector"));
    }

    #[test]
    fn const_directive_resolves_symbols_and_rejects_duplicates() {
        let program = assemble(
            r#"
            .const value = $10
            .org $C000
            LDA value
            .reset $C000
        "#,
        )
        .expect("const should resolve");
        assert_eq!(program.bytes.get(&0xC000), Some(&0xA5));
        assert_eq!(program.bytes.get(&0xC001), Some(&0x10));

        let duplicate = assemble(
            r#"
            .const COUNT = 1
            .const count = 2
            .org $C000
            .reset $C000
        "#,
        )
        .expect_err("duplicate const should fail");
        assert!(matches!(duplicate, DslError::DuplicateConst(name) if name == "COUNT"));
    }

    #[test]
    fn addressing_mode_selection_prefers_zeropage_when_possible() {
        let program = assemble(
            r#"
            .org $C000
            LDA $10,X
            LDA $1234,X
            LDX $20,Y
            LDX $1234,Y
            LDA $30
            LDA $1234
            .reset $C000
        "#,
        )
        .expect("assembly should succeed");

        let expected = [
            (0xC000, 0xB5),
            (0xC001, 0x10),
            (0xC002, 0xBD),
            (0xC003, 0x34),
            (0xC004, 0x12),
            (0xC005, 0xB6),
            (0xC006, 0x20),
            (0xC007, 0xBE),
            (0xC008, 0x34),
            (0xC009, 0x12),
            (0xC00A, 0xA5),
            (0xC00B, 0x30),
            (0xC00C, 0xAD),
            (0xC00D, 0x34),
            (0xC00E, 0x12),
        ];
        for (addr, opcode) in expected {
            assert_eq!(
                program.bytes.get(&addr),
                Some(&opcode),
                "unexpected opcode/operand byte at ${addr:04X}"
            );
        }
    }

    #[test]
    fn branch_out_of_range_is_reported_for_immediate_target() {
        let err = assemble(
            r#"
            .org $C000
            BNE $C200
            .reset $C000
        "#,
        )
        .expect_err("out of range branch should fail");

        match err {
            DslError::BranchOutOfRange { from, to, .. } => {
                assert_eq!(from, 0xC002);
                assert_eq!(to, 0xC200);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn branch_out_of_range_is_reported_for_symbol_fixup() {
        let err = assemble(
            r#"
            .org $C000
            BNE FAR
            .org $C200
            FAR:
                NOP
            .reset $C000
        "#,
        )
        .expect_err("out of range branch fixup should fail");

        assert!(matches!(err, DslError::BranchOutOfRange { .. }));
    }

    #[test]
    fn duplicate_address_allows_same_value_and_rejects_conflict() {
        let ok = assemble(
            r#"
            .org $C000
            .byte $01
            .org $C000
            .byte $01
            .reset $C000
        "#,
        );
        assert!(ok.is_ok());

        let conflict = assemble(
            r#"
            .org $C000
            .byte $01
            .org $C000
            .byte $02
            .reset $C000
        "#,
        )
        .expect_err("conflicting duplicate address should fail");
        assert!(matches!(conflict, DslError::DuplicateAddress { .. }));
    }

    #[test]
    fn emit_ines_rom_respects_chr_and_mirroring_layout() {
        let program = assemble(
            r#"
            .bank 1
            RESET:
                NOP
            .reset RESET
        "#,
        )
        .expect("assembly should succeed");

        let no_chr = emit_ines_nrom_rom(
            &program,
            &RomBuildOptions {
                mirroring: Mirroring::Horizontal,
                chr_rom: Vec::new(),
            },
        )
        .expect("rom without chr should emit");
        assert_eq!(no_chr[4], 1);
        assert_eq!(no_chr[5], 0);
        assert_eq!(no_chr.len(), 16 + PRG_BANK_BYTES);

        let custom_chr = emit_ines_nrom_rom(
            &program,
            &RomBuildOptions {
                mirroring: Mirroring::Vertical,
                chr_rom: vec![0xAB, 0xCD, 0xEF],
            },
        )
        .expect("rom with chr should emit");
        assert_eq!(custom_chr[4], 1);
        assert_eq!(custom_chr[5], 1);
        assert_eq!(custom_chr[6] & 0x01, 0x01);
        assert_eq!(custom_chr.len(), 16 + PRG_BANK_BYTES + CHR_BANK_BYTES);
        let chr_start = 16 + PRG_BANK_BYTES;
        assert_eq!(&custom_chr[chr_start..chr_start + 3], &[0xAB, 0xCD, 0xEF]);
        assert!(custom_chr[chr_start + 3..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn emit_ines_rom_rejects_chr_bank_count_over_header_limit() {
        let program = assemble(
            r#"
            .bank 1
            RESET:
                NOP
            .reset RESET
        "#,
        )
        .expect("assembly should succeed");

        let max_ok = emit_ines_nrom_rom(
            &program,
            &RomBuildOptions {
                mirroring: Mirroring::Horizontal,
                chr_rom: vec![0_u8; usize::from(u8::MAX) * CHR_BANK_BYTES],
            },
        )
        .expect("255 CHR banks should fit iNES header");
        assert_eq!(max_ok[5], u8::MAX);

        let too_many = emit_ines_nrom_rom(
            &program,
            &RomBuildOptions {
                mirroring: Mirroring::Horizontal,
                chr_rom: vec![0_u8; (usize::from(u8::MAX) + 1) * CHR_BANK_BYTES],
            },
        )
        .expect_err("256 CHR banks should exceed iNES header");
        assert!(
            matches!(too_many, DslError::InvalidRomLayout(message) if message.contains("exceeds iNES header limit"))
        );
    }

    #[test]
    fn unknown_or_mode_error_uses_mode_name_for_known_mnemonics() {
        let known = unknown_or_mode_error(3, "LDA", AddressingMode::Indirect);
        match known {
            DslError::UnsupportedAddressing {
                line,
                mnemonic,
                mode,
            } => {
                assert_eq!(line, 3);
                assert_eq!(mnemonic, "LDA");
                assert_eq!(mode, "indirect");
            }
            other => panic!("unexpected known mnemonic error: {other:?}"),
        }

        let unknown = unknown_or_mode_error(4, "WUT", AddressingMode::Immediate);
        match unknown {
            DslError::UnknownMnemonic { line, mnemonic } => {
                assert_eq!(line, 4);
                assert_eq!(mnemonic, "WUT");
            }
            other => panic!("unexpected unknown mnemonic error: {other:?}"),
        }
    }

    #[test]
    fn mnemonic_and_mode_helpers_cover_known_values() {
        assert!(has_mnemonic("LDA"));
        assert!(has_mnemonic("TAX"));
        assert!(!has_mnemonic("WUT"));

        assert_eq!(mode_name(AddressingMode::Implied), "implied");
        assert_eq!(mode_name(AddressingMode::Accumulator), "accumulator");
        assert_eq!(mode_name(AddressingMode::Immediate), "immediate");
        assert_eq!(mode_name(AddressingMode::ZeroPage), "zeropage");
        assert_eq!(mode_name(AddressingMode::ZeroPageX), "zeropage,X");
        assert_eq!(mode_name(AddressingMode::ZeroPageY), "zeropage,Y");
        assert_eq!(mode_name(AddressingMode::Absolute), "absolute");
        assert_eq!(mode_name(AddressingMode::AbsoluteX), "absolute,X");
        assert_eq!(mode_name(AddressingMode::AbsoluteY), "absolute,Y");
        assert_eq!(mode_name(AddressingMode::Indirect), "indirect");
        assert_eq!(mode_name(AddressingMode::IndirectX), "(indirect,X)");
        assert_eq!(mode_name(AddressingMode::IndirectY), "(indirect),Y");
        assert_eq!(mode_name(AddressingMode::Relative), "relative");
    }

    #[test]
    fn opcode_table_reports_supported_mode_pairs() {
        use AddressingMode::{
            Absolute, AbsoluteX, AbsoluteY, Accumulator, Immediate, Implied, Indirect, IndirectX,
            IndirectY, Relative, ZeroPage, ZeroPageX, ZeroPageY,
        };

        fn assert_supported_modes(mnemonic: &str, modes: &[AddressingMode]) {
            for mode in modes {
                assert!(
                    opcode_for(mnemonic, *mode).is_some(),
                    "expected opcode for {mnemonic} in mode {}",
                    mode_name(*mode)
                );
            }
        }

        assert_supported_modes(
            "ADC",
            &[
                Immediate, ZeroPage, ZeroPageX, Absolute, AbsoluteX, AbsoluteY, IndirectX,
                IndirectY,
            ],
        );
        assert_supported_modes(
            "AND",
            &[
                Immediate, ZeroPage, ZeroPageX, Absolute, AbsoluteX, AbsoluteY, IndirectX,
                IndirectY,
            ],
        );
        assert_supported_modes(
            "ASL",
            &[Accumulator, ZeroPage, ZeroPageX, Absolute, AbsoluteX],
        );
        assert_supported_modes("BCC", &[Relative]);
        assert_supported_modes("BCS", &[Relative]);
        assert_supported_modes("BEQ", &[Relative]);
        assert_supported_modes("BIT", &[ZeroPage, Absolute]);
        assert_supported_modes("BMI", &[Relative]);
        assert_supported_modes("BNE", &[Relative]);
        assert_supported_modes("BPL", &[Relative]);
        assert_supported_modes("BRK", &[Implied]);
        assert_supported_modes("BVC", &[Relative]);
        assert_supported_modes("BVS", &[Relative]);
        assert_supported_modes("CLC", &[Implied]);
        assert_supported_modes("CLD", &[Implied]);
        assert_supported_modes("CLI", &[Implied]);
        assert_supported_modes("CLV", &[Implied]);
        assert_supported_modes(
            "CMP",
            &[
                Immediate, ZeroPage, ZeroPageX, Absolute, AbsoluteX, AbsoluteY, IndirectX,
                IndirectY,
            ],
        );
        assert_supported_modes("CPX", &[Immediate, ZeroPage, Absolute]);
        assert_supported_modes("CPY", &[Immediate, ZeroPage, Absolute]);
        assert_supported_modes("DEC", &[ZeroPage, ZeroPageX, Absolute, AbsoluteX]);
        assert_supported_modes("DEX", &[Implied]);
        assert_supported_modes("DEY", &[Implied]);
        assert_supported_modes(
            "EOR",
            &[
                Immediate, ZeroPage, ZeroPageX, Absolute, AbsoluteX, AbsoluteY, IndirectX,
                IndirectY,
            ],
        );
        assert_supported_modes("INC", &[ZeroPage, ZeroPageX, Absolute, AbsoluteX]);
        assert_supported_modes("INX", &[Implied]);
        assert_supported_modes("INY", &[Implied]);
        assert_supported_modes("JMP", &[Absolute, Indirect]);
        assert_supported_modes("JSR", &[Absolute]);
        assert_supported_modes(
            "LDA",
            &[
                Immediate, ZeroPage, ZeroPageX, Absolute, AbsoluteX, AbsoluteY, IndirectX,
                IndirectY,
            ],
        );
        assert_supported_modes(
            "LDX",
            &[Immediate, ZeroPage, ZeroPageY, Absolute, AbsoluteY],
        );
        assert_supported_modes(
            "LDY",
            &[Immediate, ZeroPage, ZeroPageX, Absolute, AbsoluteX],
        );
        assert_supported_modes(
            "LSR",
            &[Accumulator, ZeroPage, ZeroPageX, Absolute, AbsoluteX],
        );
        assert_supported_modes("NOP", &[Implied]);
        assert_supported_modes(
            "ORA",
            &[
                Immediate, ZeroPage, ZeroPageX, Absolute, AbsoluteX, AbsoluteY, IndirectX,
                IndirectY,
            ],
        );
        assert_supported_modes("PHA", &[Implied]);
        assert_supported_modes("PHP", &[Implied]);
        assert_supported_modes("PLA", &[Implied]);
        assert_supported_modes("PLP", &[Implied]);
        assert_supported_modes(
            "ROL",
            &[Accumulator, ZeroPage, ZeroPageX, Absolute, AbsoluteX],
        );
        assert_supported_modes(
            "ROR",
            &[Accumulator, ZeroPage, ZeroPageX, Absolute, AbsoluteX],
        );
        assert_supported_modes("RTI", &[Implied]);
        assert_supported_modes("RTS", &[Implied]);
        assert_supported_modes(
            "SBC",
            &[
                Immediate, ZeroPage, ZeroPageX, Absolute, AbsoluteX, AbsoluteY, IndirectX,
                IndirectY,
            ],
        );
        assert_supported_modes("SEC", &[Implied]);
        assert_supported_modes("SED", &[Implied]);
        assert_supported_modes("SEI", &[Implied]);
        assert_supported_modes(
            "STA",
            &[
                ZeroPage, ZeroPageX, Absolute, AbsoluteX, AbsoluteY, IndirectX, IndirectY,
            ],
        );
        assert_supported_modes("STX", &[ZeroPage, ZeroPageY, Absolute]);
        assert_supported_modes("STY", &[ZeroPage, ZeroPageX, Absolute]);
        assert_supported_modes("TAX", &[Implied]);
        assert_supported_modes("TAY", &[Implied]);
        assert_supported_modes("TSX", &[Implied]);
        assert_supported_modes("TXA", &[Implied]);
        assert_supported_modes("TXS", &[Implied]);
        assert_supported_modes("TYA", &[Implied]);

        assert!(opcode_for("LDA", AddressingMode::Implied).is_none());
        assert!(opcode_for("WUT", AddressingMode::Immediate).is_none());
    }

    #[test]
    fn parser_helper_functions_enforce_expected_rules() {
        assert_eq!(
            strip_comments(r#"LDA #$01 ; trailing comment"#),
            "LDA #$01 "
        );
        assert_eq!(
            strip_comments(r#"LDA #"//" // real comment"#),
            r#"LDA #"//" "#
        );

        assert_eq!(split_leading_label("loop: NOP"), Some(("loop", " NOP")));
        assert_eq!(split_leading_label("bad label: NOP"), None);

        assert_eq!(
            parse_const_assignment("NAME = $10", 12).expect("const assignment"),
            ("NAME", "$10")
        );
        assert!(parse_const_assignment("NAME", 12).is_err());
        assert!(parse_const_assignment("= $10", 12).is_err());
        assert!(parse_const_assignment("NAME =", 12).is_err());

        assert!(validate_symbol("GOOD_NAME1").is_ok());
        assert!(validate_symbol("_good").is_ok());
        assert!(validate_symbol("").is_err());
        assert!(validate_symbol("1bad").is_err());
        assert!(validate_symbol("bad-name").is_err());

        assert_eq!(
            parse_expr("-$10", 1).expect("negative hex"),
            Expr::Number(-16)
        );
        assert_eq!(
            parse_expr("+0x20", 1).expect("positive hex"),
            Expr::Number(32)
        );
        assert_eq!(
            parse_expr("label_name", 1).expect("symbol parse"),
            Expr::Symbol("LABEL_NAME".to_owned())
        );
        assert!(parse_expr("-label_name", 1).is_err());
    }

    #[test]
    fn csv_and_string_literal_helpers_handle_escapes_and_errors() {
        let parts = split_csv(r#""A,B", $10, "C\"D""#).expect("csv parse");
        assert_eq!(parts, vec![r#""A,B""#, "$10", r#""C\"D""#]);

        let decoded = decode_string_literal(r#""A\n\r\t\\\"\x41""#).expect("string decode");
        assert_eq!(decoded, vec![b'A', b'\n', b'\r', b'\t', b'\\', b'"', 0x41]);

        let utf_err = decode_string_literal("\"\u{0100}\"").expect_err("out of byte range");
        assert!(utf_err.contains("outside byte range"));

        let dangling_escape =
            decode_string_literal("\"abc\\\"").expect_err("dangling escape should fail");
        assert!(dangling_escape.contains("dangling escape sequence"));

        let invalid_hex = decode_string_literal(r#""\xG1""#).expect_err("invalid hex escape");
        assert!(invalid_hex.contains("invalid hex escape sequence"));

        let invalid_hex2 = decode_string_literal(r#""\x1Z""#).expect_err("invalid hex escape");
        assert!(invalid_hex2.contains("invalid hex escape sequence"));
    }

    #[test]
    fn parse_expr_reports_unknown_mnemonic_with_one_based_line_numbers() {
        let err = assemble("WUT").expect_err("unknown mnemonic should fail");
        match err {
            DslError::UnknownMnemonic { line, mnemonic } => {
                assert_eq!(line, 1);
                assert_eq!(mnemonic, "WUT");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn branch_relative_encoding_is_correct_for_in_range_targets() {
        let mut immediate = Assembler::new(AssembleConfig::default());
        immediate.current_addr = 0xC001;
        immediate
            .emit_expr_byte(Expr::Number(0xC004), 1, FixupKind::Relative)
            .expect("immediate relative should emit");
        assert_eq!(immediate.bytes.get(&0xC001), Some(&0x02));

        let mut fixup = Assembler::new(AssembleConfig::default());
        fixup.fixups.push(Fixup {
            line: 1,
            addr: 0xC001,
            expr: Expr::Symbol("TARGET".to_owned()),
            kind: FixupKind::Relative,
        });
        fixup.labels.insert("TARGET".to_owned(), 0xC003);
        fixup.vectors.reset = Some(Expr::Number(0xC000));
        let finalized = fixup
            .finalize()
            .expect("forward relative fixup should resolve");
        assert_eq!(finalized.bytes.get(&0xC001), Some(&0x01));
    }

    #[test]
    fn emit_ines_rom_keeps_single_bank_for_upper_bank_program_bytes_only() {
        let program = assemble(
            r#"
            .bank 1
            RESET:
                LDA #$01
                NOP
            .reset RESET
        "#,
        )
        .expect("assembly should succeed");
        let rom =
            emit_ines_nrom_rom(&program, &RomBuildOptions::default()).expect("rom should emit");
        assert_eq!(
            rom[4], 1,
            "program without <$C000 bytes should stay single-bank"
        );
    }

    #[test]
    fn insert_mapped_byte_allows_vector_bytes_that_match_existing_program_data() {
        let mut bytes = BTreeMap::new();
        bytes.insert(VEC_RESET, 0x34);
        bytes.insert(VEC_RESET + 1, 0x12);
        let program = AssembledProgram {
            default_org: 0xC000,
            bytes,
            labels: BTreeMap::new(),
            nmi_vector: 0x1234,
            reset_vector: 0x1234,
            irq_vector: 0x1234,
        };
        let rom = emit_ines_nrom_rom(&program, &RomBuildOptions::default())
            .expect("matching vector bytes should not conflict");
        assert_eq!(rom[4], 1);
    }

    #[test]
    fn strip_comments_handles_string_boundaries_and_trailing_slashes() {
        assert_eq!(
            strip_comments(r#".text "a;still" ; comment"#),
            r#".text "a;still" "#
        );
        assert_eq!(strip_comments("/"), "/");
        assert_eq!(strip_comments("/x"), "/x");
        assert_eq!(strip_comments("x//comment"), "x");
    }

    #[test]
    fn quoted_string_predicate_requires_both_delimiters() {
        assert!(is_quoted_string(r#""ok""#));
        assert!(!is_quoted_string(r#""missing-end"#));
        assert!(!is_quoted_string(r#"missing-start""#));
    }

    #[test]
    fn decode_string_literal_accepts_byte_boundary_character_ff() {
        let decoded = decode_string_literal("\"\u{00FF}\"").expect("0xFF should be valid");
        assert_eq!(decoded, vec![0xFF]);
    }
}
