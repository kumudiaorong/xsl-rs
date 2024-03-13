mod common;
#[test]
fn rbtree() {
    let test = || {
        let mut tree = xsl_rs::collections::RBTreeMap::new();
        let data = common::rand_data(10000, 0..1000000);
        for k in data.iter() {
            tree.insert(k.clone(), 0);
        }
        for k in &data {
            tree.remove(k);
        }
    };
    common::repeat(test, 10);
}
