/// Helper function to parse command-line arguments in the format `--flag` or `--flag=value`
pub fn parse_arg<T, F, P>(
    args: &[String],
    idx: &mut usize,
    flag: &str,
    mut apply: F,
    parse: P,
) -> Result<bool, String>
where
    F: FnMut(T),
    P: Fn(&str, &str) -> Result<T, String>,
{
    let arg = &args[*idx];
    if arg == flag {
        let Some(val) = args.get(*idx + 1) else {
            return Err(format!("missing value after {flag}"));
        };
        apply(parse(val, flag)?);
        *idx += 2;
        Ok(true)
    } else if let Some(val) = arg.strip_prefix(&format!("{flag}=")) {
        if val.is_empty() {
            return Err(format!("missing value after {flag}="));
        }
        apply(parse(val, flag)?);
        *idx += 1;
        Ok(true)
    } else {
        Ok(false)
    }
}
