use rand::{distributions::Alphanumeric, Rng};

/// Returns a random string of length ``length``
pub fn gen_random(length: usize) -> String {
    let s: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();

    s
}
