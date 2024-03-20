mod common;
#[test]
fn rbtree() {
    let test = || {
        let mut tree = xsl::collections::RBTreeMap::new();
        let data = common::rand_data(1, 0..1000000);
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
    let mut tree = xsl::collections::RBTreeMap::new();
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
}

#[test]
fn fuzzy_finder() {
    let mut finder = xsl::collections::FuzzyFinder::default();
    finder.insert("hello".to_string(), 1);
    assert_eq!(finder.search("hello".to_string()), Some(vec![&1]));
    assert_eq!(finder.search("world".to_string()), None);
    finder.insert("ello".to_string(), 2);
    assert_eq!(finder.search_prefix("he".to_string()), Some(vec![&1]));
    assert_eq!(finder.search_prefix("e".to_string()), Some(vec![&2, &1]));
    assert_eq!(finder.search_prefix("w".to_string()), None);
}
