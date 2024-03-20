use super::flag::{toggle_rela, Flag, LEFT, RIGHT};
use crate::alloc::{handle_alloc_error, Allocator};
use core::alloc::Layout;
use core::borrow::Borrow;
use core::fmt::{Debug, Display};
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

#[allow(dead_code)]
#[cfg(debug_assertions)]
fn debug_next<K, V>(node: &OwnedNodeRef<K, V>) -> [Option<&K>; 3] {
    let get_key = |node: &Option<NonNull<Node<K, V>>>| {
        node.as_ref().map(|x| &unsafe { x.as_ref() }.key_value.0)
    };
    [
        get_key(&node.next[0].node),
        get_key(&node.next[1].node),
        get_key(&Some(node.parent.node)),
    ]
}
pub struct Node<K, V> {
    pub key_value: (K, V),
    pub next: [NodeRef<K, V>; 2],
    pub parent: OwnedNodeRef<K, V>,
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
    pub fn into_owned(self) -> Option<OwnedNodeRef<K, V>> {
        self.node.map(|node| OwnedNodeRef { node })
    }
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
pub enum SearchResult<K, V> {
    Found(OwnedNodeRef<K, V>),
    NotFound(OwnedNodeRef<K, V>, u8),
}

#[derive(Copy)]
pub struct OwnedNodeRef<K, V> {
    pub(super) node: NonNull<Node<K, V>>,
}
impl<K, V> Clone for OwnedNodeRef<K, V> {
    fn clone(&self) -> Self {
        Self { node: self.node }
    }
}
impl<K, V> OwnedNodeRef<K, V> {
    pub fn get_node_ref(&self) -> NodeRef<K, V> {
        NodeRef {
            node: Some(self.node),
        }
    }
    #[inline(always)]
    pub fn set_child(&mut self, mut child: OwnedNodeRef<K, V>, rela: u8) {
        self.next[rela as usize] = child.get_node_ref();
        child.set_parent(self.clone(), rela);
    }
    pub unsafe fn min(&self) -> Self {
        let mut cur = self.clone();
        let mut next;
        loop {
            next = cur.next[0].clone();
            if next.is_none() {
                return cur.clone();
            }
            cur = next.into_owned().unwrap();
        }
    }
    pub unsafe fn max(&self) -> Self {
        let mut cur = self.clone();
        let mut next;
        loop {
            next = cur.next[1].clone();
            if next.is_none() {
                return cur.clone();
            }
            cur = next.into_owned().unwrap();
        }
    }

    pub unsafe fn next_unchecked(&self) -> Self {
        if self.next[1].is_none() {
            if self.flag.is_left() {
                self.parent.clone()
            } else {
                let mut node = self.parent.clone();
                while node.flag.is_right() {
                    node = node.parent.clone();
                }
                node.parent.clone()
            }
        } else {
            self.next[1].clone().into_owned().unwrap().min()
        }
    }
    pub unsafe fn next_back_unchecked(&self) -> Self {
        if self.next[0].is_none() {
            if self.flag.is_right() {
                self.parent.clone()
            } else {
                let mut node = self.parent.clone();
                while node.flag.is_left() {
                    node = node.parent.clone();
                }
                node.parent.clone()
            }
        } else {
            self.next[0].clone().into_owned().unwrap().max()
        }
    }
    pub fn unwrap(&self) -> NonNull<Node<K, V>> {
        self.node
    }
    pub fn into_mut<'a>(mut self) -> &'a mut Node<K, V> {
        unsafe { self.node.as_mut() }
    }
    pub fn into_ref<'a>(self) -> &'a Node<K, V> {
        unsafe { self.node.as_ref() }
    }
    pub fn into_ref_key_value<'a>(self) -> (&'a K, &'a V) {
        let node = self.into_ref();
        (&node.key_value.0, &node.key_value.1)
    }
    pub fn search<Q>(&self, key: &Q) -> SearchResult<K, V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        let mut cur;
        let mut last = self.clone();
        let mut rela;
        loop {
            rela = match key.borrow().cmp(last.key_value.0.borrow()) {
                core::cmp::Ordering::Equal => return SearchResult::Found(last.clone()),
                core::cmp::Ordering::Less => {
                    cur = last.next[0].clone();
                    LEFT
                }
                core::cmp::Ordering::Greater => {
                    cur = last.next[1].clone();
                    RIGHT
                }
            };
            if cur.is_none() {
                return SearchResult::NotFound(last.clone(), rela);
            }
            last = cur.into_owned().unwrap();
        }
    }
    pub fn rest_double_red_adjust(&mut self) -> Option<OwnedNodeRef<K, V>> {
        let mut new_root = None;
        let mut child_node = self.clone();
        let mut parent_node = child_node.parent.clone();
        let mut gparent = parent_node.parent.clone();
        let prela = parent_node.flag.rela();
        let mut uncle = gparent.next[toggle_rela(prela)].clone();
        gparent.flag.set_red();

        if uncle.flag.is_black() {
            let grela = gparent.flag.rela();
            let crela = child_node.flag.rela();
            if child_node.flag.rela() != prela {
                if gparent.flag.is_root() {
                    child_node.flag.set_root();
                    new_root = Some(child_node.clone());
                } else {
                    let mut next_parent = gparent.parent.clone();
                    next_parent.next[grela] = child_node.get_node_ref();
                    child_node.set_parent(next_parent, grela as u8);
                }
                let mut lgchild = child_node.next[crela].clone();
                let mut rgchild = child_node.next[prela].clone();
                gparent.set_parent(child_node.clone(), crela as u8);
                gparent.next[prela] = lgchild.clone();
                lgchild.set_parent(gparent.clone(), prela as u8);
                child_node.next[crela] = gparent.get_node_ref();
                child_node.next[prela] = parent_node.get_node_ref();
                rgchild.set_parent(parent_node.clone(), crela as u8);
                parent_node.set_parent(child_node.clone(), prela as u8);
                parent_node.next[crela] = rgchild;
                parent_node = child_node;
            } else {
                if gparent.flag.is_root() {
                    parent_node.flag.set_root();
                    new_root = Some(parent_node.clone());
                } else {
                    gparent.parent.set_child(parent_node.clone(), grela as u8);
                }
                let toggle_grela = toggle_rela(crela);
                gparent.set_child(
                    parent_node.next[toggle_grela].clone().into_owned().unwrap(),
                    crela as u8,
                );
                parent_node.set_child(gparent, toggle_grela as u8);
            }
        } else {
            uncle.flag.set_black();
            if gparent.flag.is_root() {
                gparent.flag.set_black();
            } else if gparent.parent.flag.is_red() {
                if let Some(nr) = gparent.rest_double_red_adjust() {
                    new_root = Some(nr);
                }
            }
        }
        parent_node.flag.set_black();
        new_root
    }
    pub fn double_red_adjust(&mut self) -> Option<OwnedNodeRef<K, V>> {
        let mut new_root = None;
        let mut child_node: OwnedNodeRef<K, V> = self.clone();
        let mut parent_node = child_node.parent.clone();
        let mut gparent = parent_node.parent.clone();
        let prela = parent_node.flag.rela();
        let mut uncle = gparent.next[toggle_rela(prela)].clone();
        gparent.flag.set_red();
        if uncle.is_none() {
            let crela = child_node.flag.rela();
            let grela = gparent.flag.rela();
            if crela != prela {
                if gparent.flag.is_root() {
                    child_node.flag.set_root();
                    new_root = Some(child_node.clone());
                } else {
                    gparent.parent.set_child(child_node.clone(), grela as u8);
                }
                child_node.set_child(parent_node.clone(), prela as u8);
                child_node.set_child(gparent.clone(), crela as u8);
                parent_node.next[crela] = NodeRef::none();
                gparent.next[prela] = NodeRef::none();
                parent_node = child_node;
            } else {
                if gparent.flag.is_root() {
                    parent_node.flag.set_root();
                    new_root = Some(parent_node.clone());
                } else {
                    gparent.parent.set_child(parent_node.clone(), grela as u8);
                }
                gparent.next[crela] = NodeRef::none();
                parent_node.set_child(gparent, toggle_rela(crela) as u8);
            }
        } else {
            uncle.flag.set_black();
            if gparent.flag.is_root() {
                gparent.flag.set_black();
            } else if gparent.parent.flag.is_red() {
                if let Some(nr) = gparent.rest_double_red_adjust() {
                    new_root = Some(nr);
                }
            }
        }
        parent_node.flag.set_black();
        new_root
    }
    pub(super) fn rasie(&mut self, rela: usize) -> Option<OwnedNodeRef<K, V>> {
        let mut new_root = None;
        let parent = self;
        let mut brother = parent.next[toggle_rela(rela)].clone().into_owned().unwrap();
        if brother.flag.is_black() {
            let mut right_nephew = brother.next[toggle_rela(rela)]
                .clone()
                .into_owned()
                .unwrap();
            let mut left_nephew = brother.next[rela].clone().into_owned().unwrap();
            if right_nephew.flag.is_red() {
                if parent.flag.is_root() {
                    parent.flag.clear_root();
                    brother.flag.set_root();
                    new_root = Some(brother.clone());
                } else {
                    let prela = parent.flag.rela();
                    let mut next_parent = parent.parent.clone();
                    next_parent.next[prela] = brother.get_node_ref();
                    brother.set_parent(next_parent, prela as u8);
                }
                brother.next[rela] = parent.get_node_ref();
                brother.flag.set_color(parent.flag.color());
                left_nephew.set_parent(parent.clone(), toggle_rela(rela) as u8);
                parent.set_parent(brother.clone(), rela as u8);
                parent.next[toggle_rela(rela)] = left_nephew.get_node_ref();
                parent.flag.set_black();
                right_nephew.flag.set_black();
            } else if left_nephew.flag.is_red() {
                if parent.flag.is_root() {
                    parent.flag.clear_root();
                    left_nephew.flag.set_root();
                    new_root = Some(left_nephew.clone());
                } else {
                    let prela = parent.flag.rela();
                    let mut next_parent = parent.parent.clone();
                    next_parent.next[prela] = left_nephew.get_node_ref();
                    left_nephew.set_parent(next_parent, prela as u8);
                }
                let mut rgnephew = left_nephew.next[toggle_rela(rela)]
                    .clone()
                    .into_owned()
                    .unwrap();
                brother.set_parent(left_nephew.clone(), toggle_rela(rela) as u8);
                brother.next[rela] = rgnephew.get_node_ref();
                rgnephew.set_parent(brother.clone(), rela as u8);
                left_nephew.next[toggle_rela(rela)] = brother.get_node_ref();
                let mut lgnephew = left_nephew.next[rela].clone().into_owned().unwrap();
                left_nephew.next[rela] = parent.get_node_ref();
                lgnephew.set_parent(parent.clone(), toggle_rela(rela) as u8);
                parent.set_parent(left_nephew.clone(), rela as u8);
                parent.next[toggle_rela(rela)] = lgnephew.get_node_ref();
                left_nephew.flag.set_color(parent.flag.color());
                parent.flag.set_black();
            } else {
                brother.flag.set_red();
                if parent.flag.is_red() {
                    parent.flag.set_black();
                } else if !parent.flag.is_root() {
                    let prela = parent.flag.rela();
                    if let Some(nr) = parent.parent.rasie(prela) {
                        new_root = Some(nr);
                    }
                }
            }
        } else {
            if parent.flag.is_root() {
                parent.flag.clear_root();
                brother.flag.set_root();
                new_root = Some(brother.clone());
            } else {
                let prela = parent.flag.rela();
                let mut next_parent = parent.parent.clone();
                next_parent.next[prela] = brother.get_node_ref();
                brother.set_parent(next_parent, prela as u8);
                brother.flag.set_black();
            }
            let mut nephew = brother.next[rela].clone().into_owned().unwrap();
            parent.next[toggle_rela(rela)] = nephew.get_node_ref();
            parent.set_parent(brother.clone(), rela as u8);
            brother.next[rela] = parent.get_node_ref();
            nephew.set_parent(parent.clone(), toggle_rela(rela) as u8);
            parent.flag.set_red();
            if let Some(nr) = parent.rasie(rela) {
                new_root = Some(nr);
            }
        }
        new_root
    }
}
impl<K, V> Deref for OwnedNodeRef<K, V> {
    type Target = Node<K, V>;
    fn deref(&self) -> &Self::Target {
        unsafe { self.node.as_ref() }
    }
}
impl<K, V> DerefMut for OwnedNodeRef<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.node.as_mut() }
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
            if self.flag.is_root() {
                &self.key_value.0
            } else {
                &self.parent.key_value.0
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
    pub fn set_parent(&mut self, parent: OwnedNodeRef<K, V>, rela: u8) {
        self.parent = parent;
        self.flag.set_rela(rela);
    }
    // pub fn single_rotate(&mut self) {
    //     let mut parent = self.parent.clone();
    //     let mut child = NodeRef {
    //         node: NonNull::new(self as *const _ as *mut _),
    //     };
    //     let crela = child.flag.rela();
    //     let mut gchild = child.next[toggle_rela(crela)].clone();
    //     if gchild.is_some() {
    //         gchild.set_parent(parent.clone(), crela as u8);
    //     }

    //     parent.next[crela] = gchild;
    //     let prela = parent.flag.rela();
    //     let mut gparent = parent.parent.clone();

    //     gparent.next[prela] = child.clone();
    //     parent.set_parent(child.clone(), toggle_rela(crela) as u8);
    //     child.next[toggle_rela(crela)] = parent;
    //     child.set_parent(gparent, prela as u8);
    // }
    // pub(super) fn rasie(&mut self, rela: usize) {
    //     let parent = self;
    //     let brother_ptr = parent.next[toggle_rela(rela)].clone();
    //     let mut brother = brother_ptr;
    //     if brother.flag.is_black() {
    //         let right_nephew_ptr = brother.next[toggle_rela(rela)].clone();
    //         let mut right_nephew = right_nephew_ptr;
    //         let left_nephew_ptr = brother.next[rela].clone();
    //         let mut left_nephew = left_nephew_ptr;
    //         if right_nephew.flag.is_red() {
    //             brother.single_rotate();
    //             brother.flag.set_color(parent.flag.color());
    //             parent.flag.set_black();
    //             right_nephew.flag.set_black();
    //         } else if left_nephew.flag.is_red() {
    //             left_nephew.single_rotate();
    //             left_nephew.single_rotate();
    //             left_nephew.flag.set_color(parent.flag.color());
    //             parent.flag.set_black();
    //         } else {
    //             brother.flag.set_red();
    //             if parent.flag.is_red() {
    //                 parent.flag.set_black();
    //             } else if parent.flag.rela() != PARENT as usize {
    //                 parent.parent.rasie(parent.flag.rela());
    //             }
    //         }
    //     } else {
    //         brother.single_rotate();
    //         parent.flag.set_red();
    //         brother.flag.set_black();
    //         parent.rasie(rela);
    //     }
    // }
}
