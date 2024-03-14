extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
extern crate std;
use crate::collections::RBTreeMap;
use std::collections::HashSet;
#[derive(Debug)]
pub struct Finder<T> {
    root: Node<T>,
    miss_count: usize,
    ignore_case: bool,
}
impl<T> Default for Finder<T> {
    fn default() -> Self {
        Self::new(3, true)
    }
}
impl<T> Extend<(String, T)> for Finder<T> {
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
impl<T> Finder<T> {
    pub fn new(miss_count: usize, ignore_case: bool) -> Self {
        Self {
            root: Node {
                children: RBTreeMap::new(),
                values: Vec::new(),
            },
            miss_count,
            ignore_case,
        }
    }
    /// Insert a word and its value into the FuzzyFinder.
    pub fn insert(&mut self, mut word: String, value: T) {
        let mut node = &mut self.root;
        if self.ignore_case {
            word = word.to_lowercase();
        }
        for c in word.chars() {
            node = node.children.entry(c).or_insert(Node {
                children: RBTreeMap::new(),
                values: Vec::new(),
            });
        }
        node.values.push(value);
    }
    /// Search a word in the FuzzyFinder.
    /// Return the value if the word is found, otherwise return None.
    /// # Example
    /// ```
    /// use xsl::collections::FuzzyFinder;
    /// let mut finder = FuzzyFinder::default();
    /// finder.insert("hello".to_string(), 1);
    /// assert_eq!(finder.search("hello".to_string()), Some(vec![&1]));
    /// assert_eq!(finder.search("world".to_string()), None);
    /// ```
    pub fn search(&self, mut word: String) -> Option<Vec<&T>> {
        if self.ignore_case {
            word = word.to_lowercase();
        }
        let mut node = &self.root;
        for c in word.chars() {
            if let Some(n) = node.children.get(&c) {
                node = n;
            } else {
                return None;
            }
        }
        (!node.values.is_empty()).then(|| node.values.iter().collect())
    }
    /// Search a word prefix in the FuzzyFinder.
    /// Return the values if the word prefix is found, otherwise return None.
    /// # Example
    /// ```
    /// use xsl::collections::FuzzyFinder;
    /// let mut finder = FuzzyFinder::default();
    /// finder.insert("hello".to_string(), 1);
    /// finder.insert("ello".to_string(), 2);
    /// assert_eq!(finder.search_prefix("he".to_string()), Some(vec![&1]));
    /// assert_eq!(finder.search_prefix("e".to_string()), Some(vec![&2, &1]));
    /// assert_eq!(finder.search_prefix("w".to_string()), None);
    /// ```
    pub fn search_prefix(&self, mut word: String) -> Option<Vec<&T>> {
        if self.ignore_case {
            word = word.to_lowercase();
        }
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
                        if miss_count < self.miss_count {
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
        if over.is_empty() {
            return None;
        }
        Some(
            over.iter()
                .rev()
                .map(|n| n.values.iter())
                .flatten()
                .collect(),
        )
    }
}
