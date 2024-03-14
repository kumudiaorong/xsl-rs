use super::flag::Rela;
use super::{map::RBTreeMap, node::Node};
use crate::alloc::Allocator;
use crate::ptr::Ptr;
use core::borrow::Borrow;
pub enum Entry<'a, K, V, A>
where
    A: Allocator + Clone,
{
    /// Existing slot with equivalent key.
    Occupied(OccupiedEntry<'a, K, V, A>),
    /// Vacant slot (no equivalent key in the map).
    Vacant(VacantEntry<'a, K, V, A>),
}
impl<'a, K, V, A> Entry<'a, K, V, A>
where
    K: Ord,
    A: Allocator + Clone,
{
    /// Ensures a value is in the entry by inserting the default if empty, and returns
    /// a mutable reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map: RBTreeMap<&str, u32> = RBTreeMap::new();
    ///
    /// map.entry(&"poneyland").or_insert(3);
    /// assert_eq!(map.entry(&"poneyland").or_insert(0), &mut 3);
    ///
    /// *map.entry(&"poneyland").or_insert(10) *= 2;
    /// assert_eq!(map.entry(&"poneyland").or_insert(0), &mut 6);
    /// ```
    #[inline]
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default function if empty,
    /// and returns a mutable reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut tree: RBTreeMap<&str, &str> = RBTreeMap::new();
    /// let value = "hoho";
    ///
    /// tree.entry(&"poneyland").or_insert_with(|| value);
    ///
    /// assert_eq!(tree.entry(&"poneyland").or_insert("hoh"), &"hoho");
    /// ```
    #[inline]
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default()),
        }
    }

    /// Ensures a value is in the entry by inserting, if empty, the result of the default function.
    /// This method allows for generating key-derived values for insertion by providing the default
    /// function a reference to the key that was moved during the `.entry(key)` method call.
    ///
    /// The reference to the moved key is provided so that cloning or copying the key is
    /// unnecessary, unlike with `.or_insert_with(|| ... )`.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map: RBTreeMap<&str, usize> = RBTreeMap::new();
    ///
    /// map.entry(&"poneyland")
    ///     .or_insert_with_key(|key| key.chars().count());
    ///
    /// assert_eq!(map.entry(&"poneyland").or_insert(0), &mut 9);
    /// ```
    #[inline]
    pub fn or_insert_with_key<F>(self, default: F) -> &'a mut V
    where
        F: FnOnce(&K) -> V,
    {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let value = default(entry.key());
                entry.insert(value)
            }
        }
    }
}
impl<'a, K, V, A> Entry<'a, K, V, A>
where
    A: Allocator + Clone,
{
    /// Returns a reference to this entry's key.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map: RBTreeMap<&str, u32> = RBTreeMap::new();
    /// assert_eq!(map.entry(&"poneyland").key(), &"poneyland");
    /// ```
    #[inline]
    pub fn key(&self) -> &K {
        match *self {
            Entry::Occupied(ref entry) => entry.key(),
            Entry::Vacant(ref entry) => entry.key(),
        }
    }
    /// Provides in-place mutable access to an occupied entry before any
    /// potential inserts into the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map: RBTreeMap<&str, u32> = RBTreeMap::new();
    ///
    /// map.entry(&"poneyland")
    ///     .and_modify(|e| *e += 1)
    ///     .or_insert(42);
    /// assert_eq!(map.entry(&"poneyland").or_insert(0), &mut 42);
    ///
    /// map.entry(&"poneyland")
    ///     .and_modify(|e| *e += 1)
    ///     .or_insert(42);
    /// assert_eq!(map.entry(&"poneyland").or_insert(0), &mut 43);
    /// ```
    #[inline]
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        match self {
            Entry::Occupied(mut entry) => {
                f(entry.get_mut());
                Entry::Occupied(entry)
            }
            Entry::Vacant(entry) => Entry::Vacant(entry),
        }
    }
}
pub struct OccupiedEntry<'a, K, V, A>
where
    A: Allocator + Clone,
{
    node: Ptr<Node<K, V>>,
    tree: &'a mut RBTreeMap<K, V, A>,
}
impl<'a, K, V, A> OccupiedEntry<'a, K, V, A>
where
    A: Allocator + Clone,
{
    #[inline]
    pub(super) fn new(node: Ptr<Node<K, V>>, tree: &'a mut RBTreeMap<K, V, A>) -> Self {
        OccupiedEntry { node, tree }
    }
    /// Gets a reference to the key in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map: RBTreeMap<&str, usize> = RBTreeMap::new();
    /// map.entry("poneyland").or_insert(12);
    /// assert_eq!(map.entry("poneyland").key(), &"poneyland");
    /// ```
    #[inline]
    #[must_use]
    pub fn key(&self) -> &K {
        self.node.key_value.0.borrow()
    }
    /// Take ownership of the key and value from the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::rbtree_map::Entry;
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map: RBTreeMap<&str, usize> = RBTreeMap::new();
    /// map.entry("poneyland").or_insert(12);
    ///
    /// if let Entry::Occupied(o) = map.entry("poneyland") {
    ///     // We delete the entry from the map.
    ///     o.remove_entry();
    /// }
    ///
    /// // If now try to get the value, it will panic:
    /// // println!("{}", map["poneyland"]);
    /// ```
    #[inline]
    pub fn remove_entry(self) -> (K, V) {
        self.tree.raw_remove(self.node)
    }
    /// Gets a reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::Entry;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    /// match map.entry(&"poneyland") {
    ///     Entry::Occupied(_) => panic!("entry was occupied"),
    ///     Entry::Vacant(v) => v.insert(12),
    /// };
    ///
    /// if let Entry::Occupied(o) = map.entry("poneyland") {
    ///     assert_eq!(o.get(), &12);
    /// }
    /// ```
    #[inline]
    pub fn get(&self) -> &V {
        &self.node.key_value.1
    }

    /// Gets a mutable reference to the value in the entry.
    ///
    /// If you need a reference to the `OccupiedEntry` which may outlive the
    /// destruction of the `Entry` value, see [`into_mut`].
    ///
    /// [`into_mut`]: Self::into_mut
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::Entry;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    /// match map.entry(&"poneyland") {
    ///     Entry::Occupied(_) => panic!("entry was occupied"),
    ///     Entry::Vacant(v) => v.insert(12),
    /// };
    ///
    /// assert_eq!(map["poneyland"], 12);
    /// if let Entry::Occupied(mut o) = map.entry("poneyland") {
    ///     *o.get_mut() += 10;
    ///     assert_eq!(*o.get(), 22);
    ///
    ///     // We can use the same Entry multiple times.
    ///     *o.get_mut() += 2;
    /// }
    ///
    /// assert_eq!(map["poneyland"], 24);
    /// ```
    #[inline]
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.node.key_value.1
    }

    /// Converts the `OccupiedEntry` into a mutable reference to the value in the entry
    /// with a lifetime bound to the map itself.
    ///
    /// If you need multiple references to the `OccupiedEntry`, see [`get_mut`].
    ///
    /// [`get_mut`]: Self::get_mut
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::Entry;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    /// match map.entry(&"poneyland") {
    ///     Entry::Occupied(_) => panic!("entry was occupied"),
    ///     Entry::Vacant(v) => v.insert(12),
    /// };
    ///
    /// assert_eq!(map["poneyland"], 12);
    /// if let Entry::Occupied(o) = map.entry("poneyland") {
    ///     *o.into_mut() += 10;
    /// }
    ///
    /// assert_eq!(map["poneyland"], 22);
    /// ```
    #[inline]
    pub fn into_mut(self) -> &'a mut V {
        &mut self.node.into_mut().key_value.1
    }

    /// Sets the value of the entry, and returns the entry's old value.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::Entry;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    /// match map.entry(&"poneyland") {
    ///     Entry::Occupied(_) => panic!("entry was occupied"),
    ///     Entry::Vacant(v) => v.insert(12),
    /// };
    ///
    /// if let Entry::Occupied(mut o) = map.entry("poneyland") {
    ///     assert_eq!(o.insert(15), 12);
    /// }
    ///
    /// assert_eq!(map["poneyland"], 15);
    /// ```
    #[inline]
    pub fn insert(&mut self, mut value: V) -> V {
        core::mem::swap(&mut self.node.key_value.1, &mut value);
        value
    }
}

pub struct VacantEntry<'a, K, V, A>
where
    A: Allocator + Clone,
{
    key: K,
    node: Ptr<Node<K, V>>,
    rela: Rela,
    tree: &'a mut RBTreeMap<K, V, A>,
}

impl<'a, K, V, A> VacantEntry<'a, K, V, A>
where
    A: Allocator + Clone,
{
    pub(super) fn new(
        key: K,
        node: Ptr<Node<K, V>>,
        rela: Rela,
        tree: &'a mut RBTreeMap<K, V, A>,
    ) -> Self {
        VacantEntry {
            key,
            node,
            rela,
            tree,
        }
    }
}
impl<'a, K, V, A> VacantEntry<'a, K, V, A>
where
    A: Allocator + Clone,
{
    /// Gets a reference to the key that would be used when inserting a value
    /// through the VacantEntry.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map: RBTreeMap<&str, usize> = RBTreeMap::new();
    /// assert_eq!(map.entry("poneyland").key(), &"poneyland");
    /// ```
    pub fn key(&self) -> &K {
        self.key.borrow()
    }
}
impl<'a, K, V, A> VacantEntry<'a, K, V, A>
where
    A: Allocator + Clone,
{
    /// Sets the value of the entry with the `VacantEntry`'s key,
    /// and returns a mutable reference to it.
    ///
    /// # Examples
    ///
    /// ```
    /// use xsl::collections::rbtree_map::Entry;
    /// use xsl::collections::RBTreeMap;
    ///
    /// let mut map: RBTreeMap<&str, u32> = RBTreeMap::new();
    ///
    /// if let Entry::Vacant(o) = map.entry("poneyland") {
    ///     o.insert(37);
    /// }
    /// assert_eq!(map["poneyland"], 37);
    /// ```
    pub fn insert(self, value: V) -> &'a mut V
    where
        K: Ord,
    {
        let node = self
            .tree
            .raw_insert(self.node, self.rela, (self.key, value));
        &mut node.into_mut().key_value.1
    }
}
