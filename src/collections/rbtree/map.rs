mod values;
use super::{
    entry::{Entry, OccupiedEntry, VacantEntry},
    flag::Color,
    iter::{Iter, IterMut},
    node::{Node, NodeRef, SearchResult},
};
use crate::{
    alloc::{Allocator, Global},
    collections::rbtree::{
        flag::{toggle_rela, LEFT, RIGHT, ROOT},
        node::OwnedNodeRef,
    },
};

use core::{
    alloc::Layout,
    borrow::Borrow,
    cmp::Ordering,
    fmt::{Debug, Display},
    ops::Index,
};
use values::{Values, ValuesMut};

pub(super) enum NodeDesc<K, V> {
    Found(OwnedNodeRef<K, V>),
    NotFound(NdNotFound<K, V>),
}

pub(super) enum NdNotFound<K, V> {
    Normal(OwnedNodeRef<K, V>, u8),
    Root,
}

pub struct RBTreeMap<K, V, A = Global>
where
    A: Allocator + Clone,
{
    pub(super) root: NodeRef<K, V>,
    pub(super) alloc: A,
    pub(super) length: usize,
}
impl<K, V, const N: usize> From<[(K, V); N]> for RBTreeMap<K, V>
where
    K: Ord,
{
    /// Converts a `[(K, V); N]` into a `BTreeMap<(K, V)>`.
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let map1 = RBTreeMap::from([(1, 2), (3, 4), (5, 6), (7, 8), (9, 10)]);
    /// let map2: RBTreeMap<_, _> = [(1, 2), (3, 4), (5, 6), (7, 8), (9, 10)].into();
    /// print!("{:?}", map1);
    /// assert_eq!(map1, map2);
    /// ```
    fn from(mut arr: [(K, V); N]) -> Self {
        if N == 0 {
            return RBTreeMap::new();
        }

        // use stable sort to preserve the insertion order.
        arr.sort_by(|a, b| a.0.cmp(&b.0));
        RBTreeMap::bulk_build_from_sorted_iter(arr.into_iter(), Global::default())
    }
}
impl<K, V, A> Debug for RBTreeMap<K, V, A>
where
    K: Debug,
    V: Debug,
    A: Allocator + Clone,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // write!(f, "RBTreeMap {{")?;
        // for (k, v) in self.iter() {
        //     write!(f, "{:?}: {:?},", k, v)?;
        // }
        // write!(f, "}}")
        use crate::alloc::Vec;

        let mut cur_stack = Vec::new();
        let mut next_stack = Vec::new();
        cur_stack.push(self.root.clone());
        while !cur_stack.is_empty() {
            for node in cur_stack.iter() {
                if node.is_none() {
                    write!(f, "[]")?;
                    continue;
                }
                write!(f, "{:?}", &**node)?;
                next_stack.push(node.next[0].clone());
                next_stack.push(node.next[1].clone());
            }
            cur_stack.clear();
            writeln!(f)?;
            core::mem::swap(&mut cur_stack, &mut next_stack);
        }
        Ok(())
    }
}

impl<'a, K, V, A> IntoIterator for &'a RBTreeMap<K, V, A>
where
    A: Allocator + Clone,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}
impl<K, V, A> PartialEq for RBTreeMap<K, V, A>
where
    K: PartialEq,
    V: PartialEq,
    A: Allocator + Clone,
{
    fn eq(&self, other: &RBTreeMap<K, V, A>) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == b)
    }
}

impl<K, V, A> Eq for RBTreeMap<K, V, A>
where
    K: Eq,
    V: Eq,
    A: Allocator + Clone,
{
}
impl<K, V, A> PartialOrd for RBTreeMap<K, V, A>
where
    K: PartialOrd,
    V: PartialOrd,
    A: Allocator + Clone,
{
    #[inline]
    fn partial_cmp(&self, other: &RBTreeMap<K, V, A>) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}
impl<K: Ord, V, A: Allocator + Clone> Extend<(K, V)> for RBTreeMap<K, V, A> {
    #[inline]
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |(k, v)| {
            self.insert(k, v);
        });
    }
}
impl<K, V, Q> Index<&Q> for RBTreeMap<K, V>
where
    K: Borrow<Q> + Ord,
    Q: ?Sized + Ord,
{
    type Output = V;
    /// Returns a reference to the value corresponding to the supplied key.
    ///
    /// # Panics
    ///
    /// Panics if the key is not present in the `BTreeMap`.
    fn index(&self, index: &Q) -> &Self::Output {
        self.get(index).expect("no entry found for key")
    }
}

impl<'a, K: Ord + Copy, V: Copy, A: Allocator + Clone> Extend<(&'a K, &'a V)>
    for RBTreeMap<K, V, A>
{
    fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
        self.extend(iter.into_iter().map(|(&key, &value)| (key, value)));
    }
}

impl<K, V, A> Clone for RBTreeMap<K, V, A>
where
    K: Clone,
    V: Clone,
    A: Allocator + Clone,
{
    fn clone(&self) -> Self {
        fn new_branch<K, V, A>(
            alloc: &A,
            src: OwnedNodeRef<K, V>,
            mut dst: OwnedNodeRef<K, V>,
            rela: u8,
        ) -> (OwnedNodeRef<K, V>, OwnedNodeRef<K, V>)
        where
            K: Clone,
            V: Clone,
            A: Allocator,
        {
            let src_child_ptr = src.next[rela as usize].clone().into_owned().unwrap();
            let src_child = src_child_ptr.clone();
            let mut new_node = {
                #[cfg(debug_assertions)]
                {
                    NodeRef::new_in(&alloc)
                }
                #[cfg(not(debug_assertions))]
                {
                    NodeRef::new_in(alloc.clone())
                }
            }
            .into_owned()
            .unwrap();
            new_node.init_from(&*src_child);
            dst.next[rela as usize] = new_node.get_node_ref();
            new_node.set_parent(dst, rela as u8);
            (src_child_ptr, new_node)
        }
        use crate::alloc::Vec;
        let mut new_tree = Self::new_in(self.alloc.clone());
        if self.is_empty() {
            return new_tree;
        }
        let mut stack = Vec::new();
        stack.push(new_branch(
            &new_tree.alloc,
            self.root.clone().into_owned().unwrap(),
            new_tree.root.clone().into_owned().unwrap(),
            ROOT,
        ));
        while let Some((src, dst)) = stack.pop() {
            //First determine whether the child node is empty
            //Simple performance test shows that the following code is faster than the next code
            if !src.next[0].is_none() {
                stack.push(new_branch(&new_tree.alloc, src.clone(), dst.clone(), LEFT));
            }
            if !src.next[1].is_none() {
                stack.push(new_branch(&new_tree.alloc, src, dst, RIGHT));
            }
        }
        new_tree.length = self.length;
        new_tree
    }
}

impl<K, V> Display for RBTreeMap<K, V>
where
    K: Display,
    V: Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use crate::alloc::Vec;

        let mut cur_stack = Vec::new();
        let mut next_stack = Vec::new();
        cur_stack.push(self.root.clone());
        while !cur_stack.is_empty() {
            for node in cur_stack.iter() {
                if node.is_none() {
                    write!(f, "[]")?;
                    continue;
                }
                write!(f, "{}", &**node)?;
                next_stack.push(node.next[0].clone());
                next_stack.push(node.next[1].clone());
            }
            cur_stack.clear();
            writeln!(f)?;
            core::mem::swap(&mut cur_stack, &mut next_stack);
        }
        Ok(())
    }
}

impl<K, V, A> Drop for RBTreeMap<K, V, A>
where
    A: Allocator + Clone,
{
    fn drop(&mut self) {
        self.clear();
    }
}

impl<K, V, A> RBTreeMap<K, V, A>
where
    A: Allocator + Clone,
{
    /// Clears the map, removing all elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// map.insert(1, "a");
    /// map.clear();
    /// assert!(map.is_empty());
    /// ```
    pub fn clear(&mut self) {
        if self.is_empty() {
            return;
        }
        use crate::alloc::Vec;
        let mut stack = Vec::new();
        stack.push(self.root.clone());
        while !stack.is_empty() {
            let mut node = stack.pop().unwrap();
            if node.is_none() {
                continue;
            }
            stack.push(node.next[0].clone());
            stack.push(node.next[1].clone());
            unsafe {
                core::ptr::drop_in_place(&mut node.key_value);
                self.alloc
                    .deallocate(node.unwrap().cast(), Layout::new::<Node<K, V>>());
            }
        }
        self.length = 0;
    }
    /// Returns `true` if the map contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// assert!(map.is_empty());
    /// map.insert(1, "a");
    /// assert!(!map.is_empty());
    /// ```
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Returns the number of elements in the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// assert_eq!(map.len(), 0);
    /// map.insert(1, "a");
    /// assert_eq!(map.len(), 1);
    /// ```
    #[must_use]
    pub const fn len(&self) -> usize {
        self.length
    }
    /// Returns the first entry in the map for in-place manipulation.
    /// The key of this entry is the minimum key in the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// map.insert(1, "a");
    /// map.insert(2, "b");
    /// if let Some(mut entry) = map.first_entry() {
    ///     if *entry.key() > 0 {
    ///         entry.insert("first");
    ///     }
    /// }
    /// assert_eq!(*map.get(&1).unwrap(), "first");
    /// assert_eq!(*map.get(&2).unwrap(), "b");
    /// ```
    pub fn first_entry(&mut self) -> Option<OccupiedEntry<'_, K, V, A>> {
        self.raw_first().map(|node| OccupiedEntry::new(node, self))
    }
    /// Returns the first key-value pair in the map.
    /// The key in this pair is the minimum key in the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// assert_eq!(map.first_key_value(), None);
    /// map.insert(1, "b");
    /// map.insert(2, "a");
    /// assert_eq!(map.first_key_value(), Some((&1, &"b")));
    /// ```
    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        self.raw_first().map(|node| node.into_ref_key_value())
    }
    /// Returns the last key-value pair in the map.
    /// The key in this pair is the maximum key in the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// map.insert(1, "b");
    /// map.insert(2, "a");
    /// assert_eq!(map.last_key_value(), Some((&2, &"a")));
    /// ```
    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        self.raw_last().map(|node| node.into_ref_key_value())
    }

    /// Returns the last entry in the map for in-place manipulation.
    /// The key of this entry is the maximum key in the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// map.insert(1, "a");
    /// map.insert(2, "b");
    /// if let Some(mut entry) = map.last_entry() {
    ///     if *entry.key() > 0 {
    ///         entry.insert("last");
    ///     }
    /// }
    /// assert_eq!(*map.get(&1).unwrap(), "a");
    /// assert_eq!(*map.get(&2).unwrap(), "last");
    /// ```
    pub fn last_entry(&mut self) -> Option<OccupiedEntry<'_, K, V, A>> {
        self.raw_last().map(|node| OccupiedEntry::new(node, self))
    }
    /// Removes and returns the first element in the map.
    /// The key of this element is the minimum key that was in the map.
    ///
    /// # Examples
    ///
    /// Draining elements in ascending order, while keeping a usable map each iteration.
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// map.insert(1, "a");
    /// map.insert(2, "b");
    /// while let Some((key, _val)) = map.pop_first() {
    ///     assert!(map.iter().all(|(k, _v)| *k > key));
    /// }
    /// assert!(map.is_empty());
    /// ```
    pub fn pop_first(&mut self) -> Option<(K, V)> {
        self.first_entry().map(|entry| entry.remove_entry())
    }
    /// Removes and returns the last element in the map.
    /// The key of this element is the maximum key that was in the map.
    ///
    /// # Examples
    ///
    /// Draining elements in descending order, while keeping a usable map each iteration.
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// map.insert(1, "a");
    /// map.insert(2, "b");
    /// while let Some((key, _val)) = map.pop_last() {
    ///     assert!(map.iter().all(|(k, _v)| *k < key));
    /// }
    /// assert!(map.is_empty());
    /// ```
    pub fn pop_last(&mut self) -> Option<(K, V)> {
        self.last_entry().map(|entry| entry.remove_entry())
    }
    /// Gets an iterator over the entries of the map, sorted by key.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// map.insert(3, "c");
    /// map.insert(2, "b");
    /// map.insert(1, "a");
    ///
    /// for (key, value) in map.iter() {
    ///     println!("{key}: {value}");
    /// }
    ///
    /// let (first_key, first_value) = map.iter().next().unwrap();
    /// assert_eq!((*first_key, *first_value), (1, "a"));
    /// ```
    pub fn iter(&self) -> Iter<'_, K, V> {
        if self.is_empty() {
            Iter::new_empty()
        } else {
            Iter::new(self.root.clone().into_owned().unwrap(), self.length)
        }
    }
    /// Gets a mutable iterator over the entries of the map, sorted by key.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::from([
    ///    ("a", 1),
    ///    ("b", 2),
    ///    ("c", 3),
    /// ]);
    ///
    /// // add 10 to the value if the key isn't "a"
    /// for (key, value) in map.iter_mut() {
    ///     if key != &"a" {
    ///         *value += 10;
    ///     }
    /// }
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        if self.is_empty() {
            IterMut::new_empty()
        } else {
            IterMut::new(self.root.clone().into_owned().unwrap(), self.length)
        }
    }
}
impl<K, V, A> RBTreeMap<K, V, A>
where
    K: Ord,
    A: Allocator + Clone,
{
    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned. The key is not updated, though; this matters for
    /// types that can be `==` without being identical. See the [module-level
    /// documentation] for more.
    ///
    /// [module-level documentation]: index.html#insert-and-complex-keys
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// assert_eq!(map.insert(37, "a"), None);
    /// assert_eq!(map.is_empty(), false);
    ///
    /// map.insert(37, "b");
    /// assert_eq!(map.insert(37, "c"), Some("b"));
    /// assert_eq!(map[&37], "c");
    /// ```
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.entry(key) {
            Entry::Occupied(mut e) => Some(e.insert(value)),
            Entry::Vacant(e) => {
                e.insert(value);
                None
            }
        }
    }
    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut count: RBTreeMap<&str, usize> = RBTreeMap::new();
    ///
    /// // count the number of occurrences of letters in the vec
    /// for x in ["a", "b", "a", "c", "a", "b"] {
    ///     count.entry(x).and_modify(|curr| *curr += 1).or_insert(1);
    /// }
    ///
    /// assert_eq!(count["a"], 3);
    /// assert_eq!(count["b"], 2);
    /// assert_eq!(count["c"], 1);
    /// ```
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V, A> {
        match self.raw_search(&key) {
            NodeDesc::Found(node) => Entry::Occupied(OccupiedEntry::new(node, self)),
            NodeDesc::NotFound(nd) => Entry::Vacant(VacantEntry::new(key, nd, self)),
        }
    }
}
impl<K, V, A> RBTreeMap<K, V, A>
where
    A: Allocator + Clone,
{
    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.get(&1), Some(&"a"));
    /// assert_eq!(map.get(&2), None);
    /// ```
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        match self.raw_search(key) {
            NodeDesc::Found(node) => Some(&node.into_ref().key_value.1),
            NodeDesc::NotFound(_) => None,
        }
    }
    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// map.insert(1, "a");
    /// if let Some(x) = map.get_mut(&1) {
    ///     *x = "b";
    /// }
    /// assert_eq!(map[&1], "b");
    /// ```
    // See `get` for implementation notes, this is basically a copy-paste with mut's added
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        match self.raw_search(key) {
            NodeDesc::Found(node) => Some(&mut node.into_mut().key_value.1),
            NodeDesc::NotFound(_) => None,
        }
    }

    /// Returns `true` if the map contains a value for the specified key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.contains_key(&1), true);
    /// assert_eq!(map.contains_key(&2), false);
    /// ```
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        self.get(key).is_some()
    }
    /// Returns the key-value pair corresponding to the supplied key.
    ///
    /// The supplied key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.get_key_value(&1), Some((&1, &"a")));
    /// assert_eq!(map.get_key_value(&2), None);
    /// ```
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        match self.raw_search(key) {
            NodeDesc::Found(node) => Some(node.into_ref_key_value()),
            NodeDesc::NotFound(_) => None,
        }
    }

    /// Removes a key from the map, returning the stored key and value if the key
    /// was previously in the map.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.remove_entry(&1), Some((1, "a")));
    /// assert_eq!(map.remove_entry(&1), None);
    /// ```
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        match self.raw_search(key) {
            NodeDesc::Found(node) => Some(self.raw_remove(node)),
            NodeDesc::NotFound(_) => None,
        }
    }
    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map = RBTreeMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.remove(&1), Some("a"));
    /// assert_eq!(map.remove(&1), None);
    /// ```
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        self.remove_entry(key).map(|(_, v)| v)
    }
    /// Gets an iterator over the values of the map, in order by key.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut a = RBTreeMap::new();
    /// a.insert(1, "hello");
    /// a.insert(2, "goodbye");
    ///
    /// let values: Vec<&str> = a.values().cloned().collect();
    /// assert_eq!(values, ["hello", "goodbye"]);
    /// ```
    pub fn values(&self) -> Values<'_, K, V> {
        Values::new(self.iter())
    }
    /// Gets a mutable iterator over the values of the map, in order by key.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut a = RBTreeMap::new();
    /// a.insert(1, String::from("hello"));
    /// a.insert(2, String::from("goodbye"));
    ///
    /// for value in a.values_mut() {
    ///     value.push_str("!");
    /// }
    ///
    /// let values: Vec<String> = a.values().cloned().collect();
    /// assert_eq!(values, [String::from("hello!"),
    ///                     String::from("goodbye!")]);
    /// ```
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut::new(self.iter_mut())
    }
}
impl<K, V, A> RBTreeMap<K, V, A>
where
    A: Allocator + Clone,
{
    pub(super) fn new_in(alloc: A) -> Self {
        RBTreeMap {
            root: NodeRef::none(),
            alloc,
            length: 0,
        }
    }
}
impl<K, V> RBTreeMap<K, V> {
    pub fn new() -> Self {
        let alloc = Global::default();
        let mut root = NodeRef {
            node: Some(Node::new_in(&alloc)),
        };
        root.flag.set_root();
        RBTreeMap {
            root,
            alloc,
            length: 0,
        }
    }
}
impl<K, V, A> RBTreeMap<K, V, A>
where
    A: Allocator + Clone,
{
    pub fn bulk_build_from_sorted_iter<I>(iter: I, alloc: A) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Ord,
        A: Allocator + Clone,
    {
        let mut tree = Self::new_in(alloc);
        for (k, v) in iter {
            tree.insert(k, v);
        }
        tree
    }
}

impl<K, V, A> RBTreeMap<K, V, A>
where
    A: Allocator + Clone,
{
    pub(super) fn raw_remove(&mut self, node: OwnedNodeRef<K, V>) -> (K, V) {
        let kv = unsafe { core::mem::transmute_copy(&node.key_value) };
        fn replace<K, V>(mut node: OwnedNodeRef<K, V>) -> OwnedNodeRef<K, V> {
            if node.next[1].is_none() {
                if node.next[0].is_none() {
                    return node;
                }
                unsafe {
                    core::ptr::copy_nonoverlapping(&node.next[0].key_value, &mut node.key_value, 1);
                }
                return node.next[0].clone().into_owned().unwrap();
            }
            let repl_node = unsafe { node.next[1].clone().into_owned().unwrap().min() };
            unsafe {
                core::ptr::copy_nonoverlapping(&repl_node.key_value, &mut node.key_value, 1);
            }
            return replace(repl_node);
        }
        let repl_node = replace(node);
        let mut parent = repl_node.parent.clone();
        let rela = repl_node.flag.rela();
        let color = repl_node.flag.color();
        self.length -= 1;
        unsafe {
            self.alloc
                .deallocate(repl_node.unwrap().cast(), Layout::new::<Node<K, V>>());
        }
        if color == Color::RED {
            parent.next[rela] = NodeRef::none();
            return kv;
        }
        if rela == ROOT as usize {
            self.root = NodeRef::none();
            return kv;
        }
        parent.next[rela] = NodeRef::none();
        let mut brother = parent.next[toggle_rela(rela)].clone().into_owned().unwrap();
        let prela = parent.flag.rela();
        if brother.flag.is_red() {
            if parent.flag.is_root() {
                brother.flag.set_root();
                self.root = brother.get_node_ref();
            } else {
                parent.parent.set_child(brother.clone(), prela as u8);
                brother.flag.set_black();
            }
            parent.next[toggle_rela(rela)] = NodeRef::none();
            let mut nephew = brother.next[rela].clone().into_owned().unwrap();
            match (
                nephew.next[rela].clone().into_owned(),
                nephew.next[toggle_rela(rela)].clone().into_owned(),
            ) {
                (None, _) => {
                    parent.flag.set_red();
                    nephew.set_child(parent, rela as u8);
                }
                (Some(mut lgnephew), Some(mut rgnephew)) => {
                    nephew.flag.set_red();
                    lgnephew.flag.set_black();
                    rgnephew.flag.set_black();
                    parent.flag.set_red();
                    lgnephew.set_child(parent, rela as u8);
                }
                (Some(mut lgnephew), None) => {
                    parent.next[toggle_rela(rela)] = NodeRef::none();
                    nephew.next[rela] = NodeRef::none();

                    brother.set_child(lgnephew.clone(), rela as u8);
                    lgnephew.set_child(nephew, toggle_rela(rela) as u8);
                    lgnephew.set_child(parent, rela as u8);
                }
            };
        } else {
            match (
                brother.next[rela].clone().into_owned(),
                brother.next[toggle_rela(rela)].clone().into_owned(),
            ) {
                (None, None) => {
                    brother.flag.set_red();
                    if !parent.flag.is_root() {
                        if parent.flag.is_black() {
                            if let Some(new_root) = parent.parent.rasie(prela) {
                                self.root = new_root.get_node_ref();
                            }
                        } else {
                            parent.flag.set_black();
                        }
                    }
                }
                (None, Some(mut rnephew)) => {
                    if parent.flag.is_root() {
                        brother.flag.set_root();
                        self.root = brother.get_node_ref();
                    } else {
                        parent.parent.set_child(brother.clone(), prela as u8);
                    }
                    rnephew.flag.set_color(parent.flag.color());
                    parent.next[toggle_rela(rela)] = NodeRef::none();
                    brother.set_child(parent, rela as u8);
                }
                (Some(mut lnephew), Some(mut rnephew)) => {
                    if parent.flag.is_root() {
                        brother.flag.set_root();
                        self.root = brother.get_node_ref();
                    } else {
                        parent.parent.set_child(brother.clone(), prela as u8);
                    }
                    brother.flag.set_color(parent.flag.color());
                    lnephew.flag.set_black();
                    rnephew.flag.set_black();
                    parent.flag.set_red();
                    parent.next[toggle_rela(rela)] = NodeRef::none();
                    lnephew.set_child(parent, rela as u8);
                }
                (Some(mut lnephew), None) => {
                    if parent.flag.is_root() {
                        lnephew.flag.set_root();
                        self.root = lnephew.get_node_ref();
                    } else {
                        parent.parent.set_child(lnephew.clone(), prela as u8);
                    }
                    if parent.flag.is_black() {
                        lnephew.flag.set_black();
                    } else {
                        parent.flag.set_black();
                    }
                    parent.next[toggle_rela(rela)] = NodeRef::none();
                    brother.next[rela] = NodeRef::none();
                    lnephew.set_child(brother, toggle_rela(rela) as u8);
                    lnephew.set_child(parent, rela as u8);
                }
            }
        }
        kv
    }
    pub(super) fn raw_search<Q>(&self, key: &Q) -> NodeDesc<K, V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        if self.len() == 0 {
            return NodeDesc::NotFound(NdNotFound::Root);
        }
        match self.root.clone().into_owned().unwrap().search(key) {
            SearchResult::Found(node) => NodeDesc::Found(node),
            SearchResult::NotFound(node, rela) => {
                NodeDesc::NotFound(NdNotFound::Normal(node, rela))
            }
        }
    }
    pub fn raw_first(&self) -> Option<OwnedNodeRef<K, V>> {
        if self.is_empty() {
            return None;
        }
        Some(unsafe { self.root.clone().into_owned().unwrap().min() })
    }
    pub fn raw_last(&self) -> Option<OwnedNodeRef<K, V>> {
        if self.is_empty() {
            return None;
        }
        Some(unsafe { self.root.clone().into_owned().unwrap().max() })
    }
}
mod tests;
