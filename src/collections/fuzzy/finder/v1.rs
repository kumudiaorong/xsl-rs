#[cfg(feature = "fuzzy_finder_v1_param")]
#[macro_use]
mod param {
    extern crate std;
    include!(std::concat!(
        std::env!("OUT_DIR"),
        "/fuzzy_finder_v1_param.rs"
    ));
}
#[cfg(not(feature = "fuzzy_finder_v1_param"))]
#[macro_use]
mod param {
    pub const MISS_COUNT: usize = 3;
    macro_rules! ignore_case {
        ($word:expr) => {
            $word = $word.to_lowercase();
        };
    }
}
extern crate alloc;
use crate::alloc::Vec;
use alloc::string::String;
extern crate std;
use crate::collections::RBTreeMap;
use std::collections::HashSet;
#[derive(Debug)]
pub struct FuzzyFinder<T> {
    root: Node<T>,
}
impl<T> Default for FuzzyFinder<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T> Extend<(String, T)> for FuzzyFinder<T> {
    fn extend<I: IntoIterator<Item = (String, T)>>(&mut self, iter: I) {
        for (word, value) in iter {
            self.insert(word, value);
        }
    }
}
#[derive(Debug)]
struct Node<T> {
    children: RBTreeMap<char, Node<T>>,
    values: Vec<T>,
}
impl<T> FuzzyFinder<T> {
    pub fn new() -> Self {
        Self {
            root: Node {
                children: RBTreeMap::new(),
                values: Vec::new(),
            },
        }
    }
    pub fn insert(&mut self, mut word: String, value: T) {
        let mut node = &mut self.root;
        ignore_case!(word);
        for c in word.chars() {
            node = node.children.entry(c).or_insert(Node {
                children: RBTreeMap::new(),
                values: Vec::new(),
            });
        }
        node.values.push(value);
    }
    pub fn search(&self, mut word: String) -> Option<&Vec<T>> {
        ignore_case!(word);
        let mut node = &self.root;
        for c in word.chars() {
            if let Some(n) = node.children.get(&c) {
                node = n;
            } else {
                return None;
            }
        }
        (!node.values.is_empty()).then_some(&node.values)
    }
    pub fn search_prefix(&self, mut word: String) -> Vec<&T> {
        ignore_case!(word);
        let mut result: Vec<&T> = Vec::new();
        let mut valid_nodes = Vec::new();
        let mut dedup = HashSet::new();
        let mut stack = Vec::new();
        stack.push((&self.root, word.chars(), 0));
        while let Some((cur_node, mut words, miss_count)) = stack.pop() {
            let back = words.clone();
            match words.next() {
                Some(c) => {
                    for (kw, node) in cur_node.children.iter().rev() {
                        if kw == &c {
                            stack.push((node, words.clone(), 0));
                        }
                        if miss_count < param::MISS_COUNT {
                            stack.push((node, back.clone(), miss_count + 1));
                        }
                    }
                }
                None => {
                    if dedup.insert(cur_node as *const Node<T>) {
                        valid_nodes.push(cur_node);
                    }
                }
            }
        }
        dedup.clear();
        let mut over = Vec::new();
        while let Some(n) = valid_nodes.pop() {
            if !n.values.is_empty() && dedup.insert(n as *const Node<T>) {
                over.push(n);
            }
            for node in n.children.values() {
                valid_nodes.push(node);
            }
        }
        over.iter().rev().for_each(|n| result.extend(&n.values));
        result
    }
}
