mod values;
use super::{
    entry::{Entry, OccupiedEntry, VacantEntry},
    flag::{Color, Rela},
    iter::{Iter, IterMut},
    node::Node,
};
use crate::{
    alloc::{Allocator, Global},
    ptr::Ptr,
};
use core::{
    alloc::Layout,
    borrow::Borrow,
    cmp::Ordering,
    fmt::{Debug, Display},
    ops::Index,
    ptr::NonNull,
};
use values::{Values, ValuesMut};
pub struct RBTreeMap<K, V, A = Global>
where
    A: Allocator + Clone,
{
    root: Ptr<Node<K, V>>,
    alloc: A,
    length: usize,
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
        write!(f, "RBTreeMap {{")?;
        for (k, v) in self.iter() {
            write!(f, "{:?}: {:?},", k, v)?;
        }
        write!(f, "}}")
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
            src: Ptr<Node<K, V>>,
            mut dst: Ptr<Node<K, V>>,
            rela: Rela,
        ) -> (Ptr<Node<K, V>>, Ptr<Node<K, V>>)
        where
            K: Clone,
            V: Clone,
            A: Allocator,
        {
            let src_child = src.next[rela as usize];
            let mut node = Node::new_in(alloc);
            node.init_from(src_child.get());
            dst.next[rela as usize] = node;
            node.set_parent(dst, rela);
            (src_child, node)
        }
        use crate::alloc::Vec;
        let mut new_tree = Self::new_in(self.alloc.clone());
        if self.is_empty() {
            return new_tree;
        }
        let mut stack = Vec::new();
        stack.push(new_branch(
            &new_tree.alloc,
            self.root,
            new_tree.root,
            Rela::PARENT,
        ));
        while let Some((src, dst)) = stack.pop() {
            //First determine whether the child node is empty
            //Simple performance test shows that the following code is faster than the next code
            if !src.next[0].is_null() {
                stack.push(new_branch(&new_tree.alloc, src, dst, Rela::LEFT));
            }
            if !src.next[1].is_null() {
                stack.push(new_branch(&new_tree.alloc, src, dst, Rela::RIGHT));
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
        cur_stack.push(self.root.next[2]);
        while !cur_stack.is_empty() {
            for node in cur_stack.iter() {
                if node.is_null() {
                    write!(f, "[]")?;
                    continue;
                }
                write!(f, "{}", node.get())?;
                next_stack.push(node.next[0]);
                next_stack.push(node.next[1]);
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
        unsafe {
            self.alloc.deallocate(
                NonNull::new(self.root.as_ptr().cast()).unwrap(),
                Layout::new::<Node<K, V>>(),
            );
        }
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
        stack.push(self.root.next[2]);
        while !stack.is_empty() {
            let node = stack.pop().unwrap();
            if node.is_null() {
                continue;
            }
            let ptr = node.as_ptr();
            stack.push(node.next[0]);
            stack.push(node.next[1]);
            unsafe {
                core::ptr::drop_in_place(ptr);
                self.alloc.deallocate(
                    NonNull::new(ptr.cast()).unwrap(),
                    Layout::new::<Node<K, V>>(),
                );
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
        if self.is_empty() {
            None
        } else {
            Some(OccupiedEntry::new(self.raw_first(), self))
        }
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
        if self.is_empty() {
            None
        } else {
            let kv = &self.raw_first().into_ref().key_value;
            Some((&kv.0, &kv.1))
        }
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
        if self.is_empty() {
            None
        } else {
            let kv = &self.raw_last().into_ref().key_value;
            Some((&kv.0, &kv.1))
        }
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
        if self.is_empty() {
            None
        } else {
            Some(OccupiedEntry::new(self.raw_last(), self))
        }
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
            Iter::new(self.root.next[2], self.length)
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
            IterMut::new(self.root.next[2], self.length)
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
    pub fn insert(&mut self, key: K, mut value: V) -> Option<V> {
        match self.raw_search(&key) {
            (mut node, Ok(())) => {
                core::mem::swap(&mut node.key_value.1, &mut value);
                Some(value)
            }
            (node, Err(rela)) => {
                self.raw_insert(node, rela, (key, value));
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
        let (node, result) = self.raw_search(&key);
        match result {
            Ok(()) => Entry::Occupied(OccupiedEntry::new(node, self)),
            Err(rela) => Entry::Vacant(VacantEntry::new(key, node, rela, self)),
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
        let (node, result) = self.raw_search(key);
        match result {
            Ok(()) => Some(&node.into_ref().key_value.1),
            Err(_) => None,
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
        let (node, result) = self.raw_search(key);
        match result {
            Ok(()) => Some(&mut node.into_mut().key_value.1),
            Err(_) => None,
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
        let (node, result) = self.raw_search(key);
        match result {
            Ok(()) => {
                let kv = &node.into_ref().key_value;
                Some((&kv.0, &kv.1))
            }
            Err(_) => None,
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
        let (node, result) = self.raw_search(key);
        match result {
            Ok(()) => Some(self.raw_remove(node)),
            Err(_) => None,
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
        let mut root = Node::new_in(&alloc);
        root.flag.set_root();
        RBTreeMap {
            root,
            alloc,
            length: 0,
        }
    }
}
impl<K, V> RBTreeMap<K, V> {
    pub fn new() -> Self {
        let alloc = Global::default();
        let mut root = Node::new_in(&alloc);
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
    pub(super) fn raw_remove(&mut self, node: Ptr<Node<K, V>>) -> (K, V) {
        let kv = unsafe { core::mem::transmute_copy(&node.key_value) };
        fn replace<K, V>(mut node: Ptr<Node<K, V>>) -> Ptr<Node<K, V>> {
            if node.next[1].is_null() {
                if node.next[0].is_null() {
                    return node;
                }
                unsafe {
                    core::ptr::copy_nonoverlapping(&node.next[0].key_value, &mut node.key_value, 1);
                }
                return node.next[0];
            }
            let repl_node = node.next[1].get().min();
            unsafe {
                core::ptr::copy_nonoverlapping(&repl_node.key_value, &mut node.key_value, 1);
            }
            return replace(repl_node);
        }
        let repl_node = replace(node);
        let mut parent_ptr = repl_node.next[2];
        let rela = repl_node.flag.rela();
        let color = repl_node.flag.color();
        parent_ptr.next[rela as usize] = Ptr::null();
        self.length -= 1;
        unsafe {
            self.alloc.deallocate(
                NonNull::new(repl_node.as_ptr().cast()).unwrap(),
                Layout::new::<Node<K, V>>(),
            );
        }
        if color == Color::RED || parent_ptr.flag.is_root() {
            return kv;
        }
        let mut brother_ptr = parent_ptr.next[rela.toggle() as usize];
        if brother_ptr.flag.is_red() {
            brother_ptr.single_rotate();
            let mut nephew_ptr = parent_ptr.next[rela.toggle() as usize];
            let mut gnephew_ptr = nephew_ptr.next[rela as usize];
            if gnephew_ptr.is_null() {
                nephew_ptr.single_rotate();
                parent_ptr.flag.set_red();
            } else {
                gnephew_ptr.single_rotate();
                gnephew_ptr.single_rotate();
            }
            brother_ptr.flag.set_black();
        } else {
            let mut nephew = brother_ptr.next[rela as usize];
            if nephew.is_null() {
                if parent_ptr.flag.is_red() {
                    brother_ptr.single_rotate();
                } else {
                    nephew = brother_ptr.next[rela.toggle() as usize];
                    if nephew.is_null() {
                        brother_ptr.flag.set_red();
                        if parent_ptr.flag.rela() != Rela::PARENT {
                            let rela = parent_ptr.flag.rela();
                            parent_ptr.next[2].rasie(rela);
                        }
                    } else {
                        brother_ptr.single_rotate();
                        nephew.flag.set_black();
                    }
                }
            } else {
                nephew.single_rotate();
                nephew.single_rotate();
                nephew.flag.set_color(parent_ptr.flag.color());
                parent_ptr.flag.set_black();
            }
        }
        kv
    }
    pub(super) fn raw_search<Q>(&self, key: &Q) -> (Ptr<Node<K, V>>, Result<(), Rela>)
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        if self.len() == 0 {
            return (self.root, Err(Rela::PARENT));
        }
        self.root.next[2].search(key)
    }
    pub(super) fn raw_insert(
        &mut self,
        mut parent: Ptr<Node<K, V>>,
        rela: Rela,
        kv: (K, V),
    ) -> Ptr<Node<K, V>> {
        let mut node = Node::new_in(&self.alloc);
        node.key_value = kv;
        parent.next[rela as usize] = node;
        node.set_parent(parent, rela);
        self.length += 1;
        if self.len() == 1 {
            node.flag.set_black();
        } else {
            if parent.flag.is_red() {
                node.double_red_adjust();
            }
        }
        node
    }
    pub(super) fn raw_first(&self) -> Ptr<Node<K, V>> {
        self.root.next[2].get().min()
    }
    pub(super) fn raw_last(&self) -> Ptr<Node<K, V>> {
        self.root.next[2].get().max()
    }
}
#[cfg(test)]
mod tests;
