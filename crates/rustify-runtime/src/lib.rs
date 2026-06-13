//! Runtime helpers shared by generated Rustify programs.

pub fn console_log<T: std::fmt::Debug>(value: T) {
    println!("{value:?}");
}

pub fn js_truthy(value: bool) -> bool {
    value
}
