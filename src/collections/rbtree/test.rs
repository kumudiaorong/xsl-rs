#[test]
fn large_data() {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut data = crate::alloc::Vec::new();
    for _ in 0..100000 {
        let key = rng.gen_range(0..1000000);
        data.push(key);
    }
    let mut tree = super::RBTreeMap::new();
    for k in data.iter() {
        tree.insert(k.clone(), 0);
    }
    for k in &data {
        tree.remove(k);
    }
}
