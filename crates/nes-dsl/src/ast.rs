#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Expr {
    Number(i64),
    Symbol(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OperandSyntax {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AddressingMode {
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
pub(crate) enum FixupKind {
    Byte,
    Word,
    Relative,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fixup {
    pub(crate) line: usize,
    pub(crate) addr: u16,
    pub(crate) expr: Expr,
    pub(crate) kind: FixupKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct VectorExprs {
    pub(crate) nmi: Option<Expr>,
    pub(crate) reset: Option<Expr>,
    pub(crate) irq: Option<Expr>,
}
