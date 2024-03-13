pub fn rand_data(n: usize, range: std::ops::Range<i32>) -> Vec<i32> {
    use rand::Rng;
    use std::collections::HashSet;
    let mut rng = rand::thread_rng();
    let mut doc = HashSet::new();
    let mut data = Vec::new();
    while data.len() < n {
        let key = rng.gen_range(range.clone());
        if !doc.contains(&key) {
            doc.insert(key);
            data.push(key);
        }
    }
    data
}
pub fn repeat<F>(f: F, n: usize)
where
    F: Fn(),
{
    for _ in 0..n {
        f();
    }
}

pub fn timing<F>(f: F) -> std::time::Duration
where
    F: FnOnce(),
{
    let start = std::time::Instant::now();
    f();
    start.elapsed()
}
