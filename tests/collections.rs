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

#[test]
fn rbtree_timing() {
    let mut tree = xsl_rs::collections::RBTreeMap::new();
    let data = common::rand_data(1000000, 0..1000000);
    let test_insert = || {
        for k in data.iter() {
            tree.insert(k.clone(), 0);
        }
    };
    let duration = common::timing(test_insert);
    println!("rbtree insert: {:?}", duration);
    let test_find = || {
        for k in &data {
            tree.get(k);
        }
    };
    let duration = common::timing(test_find);
    println!("rbtree find: {:?}", duration);
    let test_remove = || {
        for k in &data {
            tree.remove(k);
        }
    };
    let duration = common::timing(test_remove);
    println!("rbtree remove: {:?}", duration);
    let mut tree = std::collections::BTreeMap::new();
    let data = common::rand_data(1000000, 0..1000000);
    let test_insert = || {
        for k in data.iter() {
            tree.insert(k.clone(), 0);
        }
    };
    let duration = common::timing(test_insert);
    println!("btree insert: {:?}", duration);
    let test_find = || {
        for k in &data {
            tree.get(k);
        }
    };
    let duration = common::timing(test_find);
    println!("btree find: {:?}", duration);
    let test_remove = || {
        for k in &data {
            tree.remove(k);
        }
    };
    let duration = common::timing(test_remove);
    println!("btree remove: {:?}", duration);
}
