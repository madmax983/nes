pub struct Config {
    pub a: bool,
    pub b: bool,
    pub c: bool,
}

pub fn do_stuff(a: bool, b: bool, c: bool) {
    if a && b && c {}
}

pub fn do_stuff_with_config(cfg: Config) {
    if cfg.a && cfg.b && cfg.c {}
}
