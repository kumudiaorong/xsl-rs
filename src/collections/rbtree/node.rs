use super::flag::{toggle_rela, Flag, LEFT, PARENT, RIGHT};
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
        let node = self.into_ref();
        (&node.key_value.0, &node.key_value.1)
    }
    #[cfg(debug_assertions)]
    pub fn new_in<A>(alloc: &A) -> Self
    where
        A: Allocator,
    {
        Self {
            node: Some(Node::new_in(alloc)),
        }
    }
    #[cfg(not(debug_assertions))]
    pub fn new_in<A>(alloc: A) -> Self
    where
        A: Allocator,
    {
        Self {
            node: Some(Node::new_in(alloc)),
        }
    }
    pub fn search<Q>(&self, key: &Q) -> (NodeRef<K, V>, Result<(), u8>)
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        let mut cur = self;
        let mut last;
        let mut rela;
        loop {
            rela = match key.borrow().cmp(cur.key_value.0.borrow()) {
                core::cmp::Ordering::Equal => return (cur.clone(), Ok(())),
                core::cmp::Ordering::Less => {
                    last = cur;
                    cur = &cur.next[0];
                    LEFT
                }
                core::cmp::Ordering::Greater => {
                    last = cur;
                    cur = &cur.next[1];
                    RIGHT
                }
            };
            if cur.is_none() {
                return (last.clone(), Err(rela));
            }
        }
    }
    pub fn rest_double_red_adjust(&mut self) {
        let mut child_node = self.clone();
        let mut parent_node = child_node.next[2].clone();
        let mut gparent = parent_node.next[2].clone();
        let prela = parent_node.flag.rela();
        let mut uncle = gparent.next[toggle_rela(prela)].clone();
        gparent.flag.set_red();

        if uncle.flag.is_black() {
            let grela = gparent.flag.rela();
            let crela = child_node.flag.rela();
            let mut next_parent = gparent.next[2].clone();
            if child_node.flag.rela() != prela {
                let mut lgchild = child_node.next[crela].clone();
                let mut rgchild = child_node.next[prela].clone();
                gparent.set_parent(child_node.clone(), crela as u8);
                gparent.next[prela] = lgchild.clone();
                lgchild.set_parent(gparent.clone(), prela as u8);
                next_parent.next[grela] = child_node.clone();
                child_node.set_parent(next_parent, grela as u8);
                child_node.next[crela] = gparent.clone();
                child_node.next[prela] = parent_node.clone();
                rgchild.set_parent(parent_node.clone(), crela as u8);
                parent_node.set_parent(child_node.clone(), prela as u8);
                parent_node.next[crela] = rgchild;
                core::mem::swap(&mut child_node, &mut parent_node);
            } else {
                let toggle_grela = toggle_rela(crela);
                let mut brother = parent_node.next[toggle_grela].clone();
                gparent.set_parent(parent_node.clone(), toggle_grela as u8);
                gparent.next[crela] = brother.clone();
                brother.set_parent(gparent.clone(), crela as u8);
                next_parent.next[grela] = parent_node.clone();
                parent_node.set_parent(next_parent, grela as u8);
                parent_node.next[toggle_grela] = gparent.clone();
            }
        } else {
            uncle.flag.set_black();
            let next_parent = gparent.next[2].clone();
            if next_parent.flag.is_red() {
                gparent.rest_double_red_adjust();
            } else if gparent.flag.rela() == PARENT as usize {
                gparent.flag.set_black();
            }
        }
        parent_node.flag.set_black();
    }
    pub fn double_red_adjust(&mut self) {
        let mut child_node: NodeRef<K, V> = self.clone();
        let mut parent_node = child_node.next[2].clone();
        let mut gparent = parent_node.next[2].clone();
        let prela = parent_node.flag.rela();
        let mut uncle = gparent.next[toggle_rela(prela)].clone();
        gparent.flag.set_red();

        if uncle.is_none() {
            let crela = child_node.flag.rela();
            let grela = gparent.flag.rela();
            if crela != prela {
                let mut next_parent = gparent.next[2].clone();
                next_parent.next[grela] = child_node.clone();
                child_node.set_parent(next_parent, grela as u8);
                child_node.next[prela] = parent_node.clone();
                parent_node.set_parent(child_node.clone(), prela as u8);
                gparent.set_parent(child_node.clone(), crela as u8);
                child_node.next[crela] = gparent.clone();
                parent_node.next[crela] = NodeRef::none();
                gparent.next[prela] = NodeRef::none();
                parent_node = child_node;
            } else {
                let mut next_parent = gparent.next[2].clone();
                parent_node.next[toggle_rela(crela)] = gparent.clone();
                gparent.set_parent(parent_node.clone(), toggle_rela(crela) as u8);
                next_parent.next[grela] = parent_node.clone();
                parent_node.set_parent(next_parent, grela as u8);
                gparent.next[crela] = NodeRef::none();
            }
        } else {
            uncle.flag.set_black();
            let next_parent = gparent.next[2].clone();
            if next_parent.flag.is_red() {
                gparent.rest_double_red_adjust();
            } else if gparent.flag.rela() == PARENT as usize {
                gparent.flag.set_black();
            }
        }
        parent_node.flag.set_black();
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
    pub(super) fn new_in<A>(alloc: A) -> NonNull<Self>
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
    #[inline(always)]
    pub fn set_parent(&mut self, parent: NodeRef<K, V>, rela: u8) {
        self.next[2] = parent;
        self.flag.set_rela(rela);
    }
    pub fn single_rotate(&mut self) {
        let mut parent = self.next[2].clone();
        let mut child = NodeRef {
            node: NonNull::new(self as *const _ as *mut _),
        };
        let crela = child.flag.rela();
        let mut gchild = child.next[toggle_rela(crela)].clone();
        if gchild.is_some() {
            gchild.set_parent(parent.clone(), crela as u8);
        }

        parent.next[crela] = gchild;
        let prela = parent.flag.rela();
        let mut gparent = parent.next[2].clone();

        gparent.next[prela] = child.clone();
        parent.set_parent(child.clone(), toggle_rela(crela) as u8);
        child.next[toggle_rela(crela)] = parent;
        child.set_parent(gparent, prela as u8);
    }
    pub(super) fn rasie(&mut self, rela: usize) {
        let parent = self;
        let brother_ptr = parent.next[toggle_rela(rela)].clone();
        let mut brother = brother_ptr;
        if brother.flag.is_black() {
            let right_nephew_ptr = brother.next[toggle_rela(rela)].clone();
            let mut right_nephew = right_nephew_ptr;
            let left_nephew_ptr = brother.next[rela].clone();
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
                } else if parent.flag.rela() != PARENT as usize {
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
