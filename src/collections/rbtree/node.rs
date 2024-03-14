use super::flag::{Flag, Rela};
use crate::alloc::{handle_alloc_error, Allocator};
use core::alloc::Layout;
use core::borrow::Borrow;
use core::fmt::{Debug, Display};
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

#[cfg(debug_assertions)]
fn debug_next<K, V>(node: &Node<K, V>) -> [Option<&K>; 3] {
    let get_key = |node: &NodeRef<K, V>| {
        node.node
            .as_ref()
            .map(|x| &unsafe { x.as_ref() }.key_value.0)
    };
    [
        get_key(&node.next[0]),
        get_key(&node.next[1]),
        get_key(&node.next[2]),
    ]
}
pub struct Node<K, V> {
    pub key_value: (K, V),
    pub next: [NodeRef<K, V>; 3],
    pub flag: Flag,
}

#[derive(Copy)]
pub struct NodeRef<K, V> {
    pub(super) node: Option<NonNull<Node<K, V>>>,
}
impl<K, V> Clone for NodeRef<K, V> {
    fn clone(&self) -> Self {
        Self { node: self.node }
    }
}
impl<K, V> NodeRef<K, V> {
    pub fn none() -> Self {
        Self { node: None }
    }
    pub fn is_none(&self) -> bool {
        self.node.is_none()
    }
    pub fn is_some(&self) -> bool {
        self.node.is_some()
    }
    pub fn into_mut<'a>(self) -> &'a mut Node<K, V> {
        unsafe { self.node.unwrap().as_mut() }
    }
    pub fn into_ref<'a>(self) -> &'a Node<K, V> {
        unsafe { self.node.unwrap().as_ref() }
    }
    pub fn unwrap(&self) -> NonNull<Node<K, V>> {
        self.node.unwrap()
    }
    pub fn into_ref_key_value<'a>(self) -> (&'a K, &'a V) {
        let node = unsafe { self.node.unwrap().as_ref() };
        (&node.key_value.0, &node.key_value.1)
    }
}
impl<K, V> Deref for NodeRef<K, V> {
    type Target = Node<K, V>;
    fn deref(&self) -> &Self::Target {
        unsafe { self.node.unwrap().as_ref() }
    }
}
impl<K, V> DerefMut for NodeRef<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.node.unwrap().as_mut() }
    }
}

impl<K, V> Debug for Node<K, V>
where
    K: Debug,
    V: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "[{},{},{:?},{:?},{:?},{:?},{:?}]",
            self.flag.color(),
            self.flag.rela(),
            self.key_value.0,
            self.key_value.1,
            match self.next[0].node {
                Some(n) => &unsafe { n.as_ref() }.key_value.0,
                None => &self.key_value.0,
            },
            match self.next[1].node {
                Some(n) => &unsafe { n.as_ref() }.key_value.0,
                None => &self.key_value.0,
            },
            match self.next[2].node {
                Some(n) => &unsafe { n.as_ref() }.key_value.0,
                None => &self.key_value.0,
            }
        )
    }
}

impl<K, V> Display for Node<K, V>
where
    K: Display,
    V: Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "[{},{},{},{}]",
            self.flag.color(),
            self.flag.rela(),
            self.key_value.0,
            self.key_value.1
        )
    }
}

impl<K, V> Node<K, V>
where
    K: Clone,
    V: Clone,
{
    pub(super) fn init_from(&mut self, source: &Self) {
        self.key_value = source.key_value.clone();
        self.flag = source.flag;
    }
}
impl<K, V> Node<K, V> {
    pub(super) fn new_in<A>(alloc: &A) -> NonNull<Self>
    where
        A: Allocator,
    {
        let layout = Layout::new::<Node<K, V>>();
        let ptr = match alloc.allocate_zeroed(layout) {
            Ok(ptr) => ptr,
            Err(_) => handle_alloc_error(layout),
        };
        ptr.cast()
    }
    /// make a new node with key and value
    ///
    pub fn set_parent(&mut self, parent: NodeRef<K, V>, rela: Rela) {
        self.next[2] = parent;
        self.flag.set_rela(rela);
    }
    pub fn single_rotate(&mut self) {
        let mut parent = self.next[2].clone();
        let mut child = NodeRef {
            node: NonNull::new(self as *const _ as *mut _),
        };
        let crela = child.flag.rela();
        let mut gchild = child.next[crela.toggle() as usize].clone();
        if gchild.is_some() {
            gchild.set_parent(parent.clone(), crela);
        }

        parent.next[crela as usize] = gchild;
        let prela = parent.flag.rela();
        let mut gparent = parent.next[2].clone();

        gparent.next[prela as usize] = child.clone();
        parent.set_parent(child.clone(), crela.toggle());
        child.next[crela.toggle() as usize] = parent;
        child.set_parent(gparent, prela);
    }
    pub fn double_red_adjust(&mut self) {
        let mut child_node = self;
        let mut parent_node = &mut *child_node.next[2].clone();
        let mut gparent = parent_node.next[2].clone();
        let mut uncle = gparent.next[parent_node.flag.rela().toggle() as usize].clone();
        gparent.flag.set_red();

        if uncle.is_none() || uncle.flag.is_black() {
            if child_node.flag.rela() != parent_node.flag.rela() {
                child_node.single_rotate();
                core::mem::swap(&mut child_node, &mut parent_node);
            }
            parent_node.single_rotate();
        } else {
            uncle.flag.set_black();
            let next_parent = gparent.next[2].clone();
            if next_parent.is_none() {
                gparent.flag.set_black();
            } else if next_parent.flag.is_red() {
                gparent.double_red_adjust();
            } else if next_parent.flag.is_root() {
                gparent.flag.set_black();
            }
        }
        parent_node.flag.set_black();
    }
    pub(super) fn search<Q>(&self, key: &Q) -> (NodeRef<K, V>, Result<(), Rela>)
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        let mut cur: NodeRef<K, V> = NodeRef {
            node: NonNull::new(self as *const _ as *mut _),
        };
        let mut next;
        loop {
            let rela = match key.borrow().cmp(cur.key_value.0.borrow()) {
                core::cmp::Ordering::Equal => return (cur, Ok(())),
                core::cmp::Ordering::Less => {
                    next = cur.next[0].clone();
                    Rela::LEFT
                }
                core::cmp::Ordering::Greater => {
                    next = cur.next[1].clone();
                    Rela::RIGHT
                }
            };
            if next.is_none() {
                return (cur, Err(rela));
            }
            cur = next;
        }
    }
    pub(super) fn rasie(&mut self, rela: Rela) {
        let parent = self;
        let brother_ptr = parent.next[rela.toggle() as usize].clone();
        let mut brother = brother_ptr;
        if brother.flag.is_black() {
            let right_nephew_ptr = brother.next[rela.toggle() as usize].clone();
            let mut right_nephew = right_nephew_ptr;
            let left_nephew_ptr = brother.next[rela as usize].clone();
            let mut left_nephew = left_nephew_ptr;
            if right_nephew.flag.is_red() {
                brother.single_rotate();
                brother.flag.set_color(parent.flag.color());
                parent.flag.set_black();
                right_nephew.flag.set_black();
            } else if left_nephew.flag.is_red() {
                left_nephew.single_rotate();
                left_nephew.single_rotate();
                left_nephew.flag.set_color(parent.flag.color());
                parent.flag.set_black();
            } else {
                brother.flag.set_red();
                if parent.flag.is_red() {
                    parent.flag.set_black();
                } else if parent.flag.rela() != Rela::PARENT {
                    parent.next[2].rasie(parent.flag.rela());
                }
            }
        } else {
            brother.single_rotate();
            parent.flag.set_red();
            brother.flag.set_black();
            parent.rasie(rela);
        }
    }
    pub(super) fn min(&self) -> NodeRef<K, V> {
        let mut cur = NodeRef {
            node: NonNull::new(self as *const _ as *mut _),
        };
        let mut next = self.next[0].clone();
        while !next.is_none() {
            cur = next;
            next = cur.next[0].clone();
        }
        cur
    }
    pub(super) fn max(&self) -> NodeRef<K, V> {
        let mut cur = NodeRef {
            node: NonNull::new(self as *const _ as *mut _),
        };
        let mut next = self.next[1].clone();
        while !next.is_none() {
            cur = next;
            next = cur.next[1].clone();
        }
        cur
    }
    pub(super) fn next_unchecked(&self) -> NodeRef<K, V> {
        if self.next[1].is_none() {
            if self.flag.is_left() {
                self.next[2].clone()
            } else {
                let mut node = self.next[2].clone();
                while node.flag.is_right() {
                    node = node.next[2].clone();
                }
                node.next[2].clone()
            }
        } else {
            self.next[1].min()
        }
    }
    pub(crate) fn next_back_unchecked(&self) -> NodeRef<K, V> {
        if self.next[0].is_none() {
            if self.flag.is_right() {
                self.next[2].clone()
            } else {
                let mut node = self.next[2].clone();
                while node.flag.is_left() {
                    node = node.next[2].clone();
                }
                node.next[2].clone()
            }
        } else {
            self.next[0].max()
        }
    }
}
