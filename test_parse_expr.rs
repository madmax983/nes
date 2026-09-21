
    #[test]
    fn parse_expr_handles_various_prefixes() {
        assert_eq!(parse_expr("-5", 1).unwrap(), Expr::Number(-5));
        assert_eq!(parse_expr("+5", 1).unwrap(), Expr::Number(5));
        assert_eq!(parse_expr("$A", 1).unwrap(), Expr::Number(10));
        assert_eq!(parse_expr("-$A", 1).unwrap(), Expr::Number(-10));
        assert_eq!(parse_expr("%1010", 1).unwrap(), Expr::Number(10));
        assert_eq!(parse_expr("-%1010", 1).unwrap(), Expr::Number(-10));
        assert_eq!(parse_expr("0xA", 1).unwrap(), Expr::Number(10));
        assert_eq!(parse_expr("0XA", 1).unwrap(), Expr::Number(10));
        assert_eq!(parse_expr("-0xA", 1).unwrap(), Expr::Number(-10));
    }
