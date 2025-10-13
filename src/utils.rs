use crate::config::Config;
use rand::Rng;

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn random_code() -> String {
    let mut rng = rand::rng();
    let len = rng.random_range(7..=17usize);
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        let idx = rng.random_range(0..Config::CODE_ABC.len());
        out.push(Config::CODE_ABC[idx] as char);
    }
    out
}

pub fn cut_off(value: &mut String, len: usize) {
    let mut idx = len;
    loop {
        if value.is_char_boundary(idx) {
            break;
        }
        idx -= 1;
    }
    value.truncate(idx);
}
