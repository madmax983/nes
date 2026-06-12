use std::collections::BTreeMap;
use crate::ast::{AddressingMode, Expr, Fixup, FixupKind, OperandSyntax, VectorExprs};
use crate::parser::{
    decode_string_literal, is_quoted_string, parse_const_assignment, parse_expr,
    parse_operand_syntax, split_csv, split_head, split_leading_label, strip_comments,
    normalize_symbol, validate_symbol,
};
use crate::opcode::opcode_for;
use crate::{AssembleConfig, AssembledProgram, DslError};

pub(crate) fn unknown_or_mode_error(line: usize, mnemonic: &str, mode: AddressingMode) -> DslError {
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

pub(crate) fn has_mnemonic(mnemonic: &str) -> bool {
    let modes = [
        AddressingMode::Implied,
        AddressingMode::Accumulator,
        AddressingMode::Immediate,
        AddressingMode::ZeroPage,
        AddressingMode::ZeroPageX,
        AddressingMode::ZeroPageY,
        AddressingMode::Absolute,
        AddressingMode::AbsoluteX,
        AddressingMode::AbsoluteY,
        AddressingMode::Indirect,
        AddressingMode::IndirectX,
        AddressingMode::IndirectY,
        AddressingMode::Relative,
    ];
    modes.iter().any(|&m| opcode_for(mnemonic, m).is_some())
}

pub(crate) fn mode_name(mode: AddressingMode) -> &'static str {
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

pub(crate) fn is_branch_mnemonic(mnemonic: &str) -> bool {
    matches!(
        mnemonic,
        "BCC" | "BCS" | "BEQ" | "BMI" | "BNE" | "BPL" | "BVC" | "BVS"
    )
}

pub(crate) fn fit_u8(value: i64, line: usize) -> Result<u8, DslError> {
    u8::try_from(value).map_err(|_| DslError::ValueOutOfRange {
        line,
        value,
        width_bits: 8,
    })
}

pub(crate) fn fit_u16(value: i64, line: usize) -> Result<u16, DslError> {
    u16::try_from(value).map_err(|_| DslError::ValueOutOfRange {
        line,
        value,
        width_bits: 16,
    })
}

#[derive(Debug, Clone)]
pub(crate) struct Assembler {
    pub(crate) config: AssembleConfig,
    pub(crate) current_addr: u16,
    pub(crate) bytes: BTreeMap<u16, u8>,
    pub(crate) labels: BTreeMap<String, u16>,
    pub(crate) constants: BTreeMap<String, i64>,
    pub(crate) fixups: Vec<Fixup>,
    pub(crate) vectors: VectorExprs,
}

impl Assembler {
    pub(crate) fn new(config: AssembleConfig) -> Self {
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

    pub(crate) fn assemble_line(&mut self, line_no: usize, raw_line: &str) -> Result<(), DslError> {
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

    pub(crate) fn finalize(mut self) -> Result<AssembledProgram, DslError> {
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
            return Err(DslError::MissingResetVector);
        }
        Ok(self.config.default_org)
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
            ".const" => self.handle_const_directive(args, line_no),
            ".byte" => self.handle_byte_directive(args, line_no),
            ".word" => self.handle_word_directive(args, line_no),
            ".text" => self.handle_text_directive(args, line_no),
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

    fn handle_const_directive(&mut self, args: &str, line_no: usize) -> Result<(), DslError> {
        let (name, value_expr) = parse_const_assignment(args, line_no)?;
        let normalized = normalize_symbol(name);
        validate_symbol(&normalized).map_err(|message| DslError::Parse {
            line: line_no,
            message,
        })?;
        if self.labels.contains_key(&normalized) || self.constants.contains_key(&normalized) {
            return Err(DslError::DuplicateConst(normalized));
        }
        let value = self.resolve_expr(&parse_expr(value_expr, line_no)?)?;
        self.constants.insert(normalized, value);
        Ok(())
    }

    fn handle_byte_directive(&mut self, args: &str, line_no: usize) -> Result<(), DslError> {
        for arg in split_csv(args)? {
            if is_quoted_string(arg) {
                let bytes = decode_string_literal(arg).map_err(|message| DslError::Parse {
                    line: line_no,
                    message,
                })?;
                for byte in bytes {
                    self.emit_u8(byte)?;
                }
            } else {
                self.emit_expr_byte(parse_expr(arg, line_no)?, line_no, FixupKind::Byte)?;
            }
        }
        Ok(())
    }

    fn handle_word_directive(&mut self, args: &str, line_no: usize) -> Result<(), DslError> {
        for arg in split_csv(args)? {
            self.emit_expr_word(parse_expr(arg, line_no)?, line_no)?;
        }
        Ok(())
    }

    fn handle_text_directive(&mut self, args: &str, line_no: usize) -> Result<(), DslError> {
        let literals = split_csv(args)?;
        if literals.is_empty() {
            return Err(DslError::Parse {
                line: line_no,
                message: ".text expects at least one quoted string".to_owned(),
            });
        }
        for lit in literals {
            if !is_quoted_string(lit) {
                return Err(DslError::Parse {
                    line: line_no,
                    message: ".text accepts only quoted string literals".to_owned(),
                });
            }
            let bytes = decode_string_literal(lit).map_err(|message| DslError::Parse {
                line: line_no,
                message,
            })?;
            for byte in bytes {
                self.emit_u8(byte)?;
            }
        }
        Ok(())
    }

    fn resolve_addressing_mode(
        &self,
        mnemonic: &str,
        syntax: OperandSyntax,
    ) -> (AddressingMode, Option<FixupKind>, Option<Expr>) {
        match syntax {
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
                    && opcode_for(mnemonic, AddressingMode::ZeroPageX).is_some()
                {
                    (AddressingMode::ZeroPageX, Some(FixupKind::Byte), Some(expr))
                } else {
                    (AddressingMode::AbsoluteX, Some(FixupKind::Word), Some(expr))
                }
            }
            OperandSyntax::AbsoluteY(expr) => {
                if self.can_use_zeropage(&expr)
                    && opcode_for(mnemonic, AddressingMode::ZeroPageY).is_some()
                {
                    (AddressingMode::ZeroPageY, Some(FixupKind::Byte), Some(expr))
                } else {
                    (AddressingMode::AbsoluteY, Some(FixupKind::Word), Some(expr))
                }
            }
            OperandSyntax::AbsoluteOrZeroPage(expr) => {
                if self.can_use_zeropage(&expr)
                    && opcode_for(mnemonic, AddressingMode::ZeroPage).is_some()
                {
                    (AddressingMode::ZeroPage, Some(FixupKind::Byte), Some(expr))
                } else {
                    (AddressingMode::Absolute, Some(FixupKind::Word), Some(expr))
                }
            }
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

        let (mode, kind, expr_opt) = self.resolve_addressing_mode(&mnemonic, syntax);

        let opcode = opcode_for(&mnemonic, mode)
            .ok_or_else(|| unknown_or_mode_error(line_no, &mnemonic, mode))?;
        self.emit_u8(opcode)?;

        if let (Some(kind), Some(expr)) = (kind, expr_opt) {
            match kind {
                FixupKind::Byte => self.emit_expr_byte(expr, line_no, FixupKind::Byte)?,
                FixupKind::Word => self.emit_expr_word(expr, line_no)?,
                FixupKind::Relative => return Err(DslError::Parse { line: line_no, message: "internal error: relative mode handled earlier".to_owned() }),
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

    pub(crate) fn emit_expr_byte(
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
                        kind,
                    });
                    Ok(())
                }
            }
            (Expr::Symbol(_), FixupKind::Relative) => {
                let fixup_addr = self.current_addr;
                self.emit_u8(0)?;
                self.fixups.push(Fixup {
                    line: line_no,
                    addr: fixup_addr,
                    expr,
                    kind,
                });
                Ok(())
            }
            (_, FixupKind::Word) => return Err(DslError::Parse { line: line_no, message: "internal error: byte emitter used for word fixup".to_owned() }),
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
