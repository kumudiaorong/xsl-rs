extern crate alloc;
extern crate std;
use alloc::string::String;
use alloc::vec::Vec;
use std::collections::BTreeMap;
use std::collections::HashSet;
const MISS_COUNT: usize = 3;
const IGNORE_CASE: bool = true;
#[derive(Debug)]
pub struct Finder<T> {
    root: Node<T>,
    miss_count: usize,
    ignore_case: bool,
}
impl<T> Default for Finder<T> {
    fn default() -> Self {
        Self {
            root: Node {
                children: BTreeMap::new(),
                values: Vec::new(),
            },
            miss_count: MISS_COUNT,
            ignore_case: IGNORE_CASE,
        }
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
    children: BTreeMap<char, Node<T>>,
    values: Vec<T>,
}
impl<T> Finder<T> {
    /// Create a new FuzzyFinder with default parameters.
    pub fn new() -> Self {
        Default::default()
    }
    /// Create a new FuzzyFinder with the given parameters.
    pub fn with_params(miss_count: usize, ignore_case: bool) -> Self {
        Self {
            root: Node {
                children: BTreeMap::new(),
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
                children: BTreeMap::new(),
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
    /// assert_eq!(finder.search("hello".to_string()), vec![&1]);
    /// assert_eq!(finder.search("world".to_string()), Vec::<&i32>::new());
    /// ```
    pub fn search(&self, mut word: String) -> Vec<&T> {
        if self.ignore_case {
            word = word.to_lowercase();
        }
        let mut node = &self.root;
        for c in word.chars() {
            if let Some(n) = node.children.get(&c) {
                node = n;
            } else {
                return Vec::new();
            }
        }
        node.values.iter().collect()
    }
    /// Search a word prefix in the FuzzyFinder.
    /// Return the values if the word prefix is found, otherwise return None.
    /// # Example
    /// ```
    /// use xsl::collections::FuzzyFinder;
    /// let mut finder = FuzzyFinder::default();
    /// finder.insert("hello".to_string(), 1);
    /// finder.insert("ello".to_string(), 2);
    /// assert_eq!(finder.search_prefix("he".to_string()), vec![&1]);
    /// assert_eq!(finder.search_prefix("e".to_string()), vec![&2, &1]);
    /// assert_eq!(finder.search_prefix("w".to_string()), Vec::<&i32>::new());
    /// ```
    pub fn search_prefix(&self, mut word: String) -> Vec<&T> {
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
        over.iter()
            .rev()
            .map(|n| n.values.iter())
            .flatten()
            .collect()
    }
}
