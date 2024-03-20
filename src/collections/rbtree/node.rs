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
        get_key(&node.next[0].ptr),
        get_key(&node.next[1].ptr),
        get_key(&Some(node.parent.ptr)),
    ]
}

pub struct Node<K, V> {
    pub key_value: (K, V),
    pub next: [NodeRef<K, V>; 2],
    pub parent: OwnedNodeRef<K, V>,
    pub flag: Flag,
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
            match self.next[0].ptr {
                Some(n) => &unsafe { n.as_ref() }.key_value.0,
                None => &self.key_value.0,
            },
            match self.next[1].ptr {
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
    pub fn init_from(&mut self, source: &Self) {
        self.key_value = source.key_value.clone();
        self.flag = source.flag;
    }
}

impl<K, V> Node<K, V> {
    pub fn new_in<A>(alloc: A) -> NonNull<Self>
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
    #[inline(always)]
    pub fn set_parent(&mut self, parent: OwnedNodeRef<K, V>, rela: u8) {
        self.parent = parent;
        self.flag.set_rela(rela);
    }
}

#[derive(Copy)]
pub struct NodeRef<K, V> {
    pub(super) ptr: Option<NonNull<Node<K, V>>>,
}
impl<K, V> Clone for NodeRef<K, V> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}
impl<K, V> NodeRef<K, V> {
    pub fn none() -> Self {
        Self { ptr: None }
    }
    pub fn is_none(&self) -> bool {
        self.ptr.is_none()
    }
    pub fn is_some(&self) -> bool {
        self.ptr.is_some()
    }
    pub fn into_owned(self) -> Option<OwnedNodeRef<K, V>> {
        self.ptr.map(|ptr| OwnedNodeRef { ptr })
    }
    pub fn get_owned(&self) -> OwnedNodeRef<K, V> {
        self.ptr.map(|ptr| OwnedNodeRef { ptr }).unwrap()
    }
}
impl<K, V> Deref for NodeRef<K, V> {
    type Target = Node<K, V>;
    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.unwrap().as_ref() }
    }
}
pub enum SearchResult<K, V> {
    Found(OwnedNodeRef<K, V>),
    NotFound(OwnedNodeRef<K, V>, u8),
}

#[derive(Copy)]
pub struct OwnedNodeRef<K, V> {
    pub(super) ptr: NonNull<Node<K, V>>,
}
impl<K, V> Clone for OwnedNodeRef<K, V> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}
impl<K, V> OwnedNodeRef<K, V> {
    #[cfg(debug_assertions)]
    pub fn new_in<A>(alloc: &A) -> Self
    where
        A: Allocator,
    {
        Self {
            ptr: Node::new_in(alloc),
        }
    }
    #[cfg(not(debug_assertions))]
    pub fn new_in<A>(alloc: A) -> Self
    where
        A: Allocator,
    {
        Self {
            ptr: Node::new_in(alloc),
        }
    }
    pub fn get_node_ref(&self) -> NodeRef<K, V> {
        NodeRef {
            ptr: Some(self.ptr),
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
            self.next[1].get_owned().min()
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
            self.next[0].get_owned().max()
        }
    }
    pub fn unwrap(&self) -> NonNull<Node<K, V>> {
        self.ptr
    }
    pub fn into_mut<'a>(mut self) -> &'a mut Node<K, V> {
        unsafe { self.ptr.as_mut() }
    }
    pub fn into_ref<'a>(self) -> &'a Node<K, V> {
        unsafe { self.ptr.as_ref() }
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
        let mut child = self.clone();
        let mut parent = child.parent.clone();
        let mut gparent = parent.parent.clone();
        let prela = parent.flag.rela();
        let mut uncle = gparent.next[toggle_rela(prela) as usize].get_owned();
        gparent.flag.set_red();

        if uncle.flag.is_black() {
            let grela = gparent.flag.rela();
            let crela = child.flag.rela();
            if child.flag.rela() != prela {
                if gparent.flag.is_root() {
                    child.flag.set_root();
                    new_root = Some(child.clone());
                } else {
                    gparent.parent.set_child(child.clone(), grela);
                }
                gparent.set_child(child.next[crela as usize].get_owned(), prela);
                parent.set_child(child.next[prela as usize].get_owned(), crela);
                child.set_child(gparent.clone(), crela);
                child.set_child(parent.clone(), prela);
                parent = child;
            } else {
                if gparent.flag.is_root() {
                    parent.flag.set_root();
                    new_root = Some(parent.clone());
                } else {
                    gparent.parent.set_child(parent.clone(), grela);
                }
                let toggle_grela = toggle_rela(crela);
                gparent.set_child(parent.next[toggle_grela as usize].get_owned(), crela);
                parent.set_child(gparent, toggle_grela);
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
        parent.flag.set_black();
        new_root
    }
    pub fn double_red_adjust(&mut self) -> Option<OwnedNodeRef<K, V>> {
        let mut new_root = None;
        let mut child = self.clone();
        let mut parent = child.parent.clone();
        let mut gparent = parent.parent.clone();
        let prela = parent.flag.rela();
        let uncle = gparent.next[toggle_rela(prela) as usize].clone();
        gparent.flag.set_red();
        if uncle.is_none() {
            let crela = child.flag.rela();
            let grela = gparent.flag.rela();
            if crela != prela {
                if gparent.flag.is_root() {
                    child.flag.set_root();
                    new_root = Some(child.clone());
                } else {
                    gparent.parent.set_child(child.clone(), grela);
                    child.flag.set_black();
                }
                child.set_child(parent.clone(), prela);
                child.set_child(gparent.clone(), crela);
                parent.next[crela as usize] = NodeRef::none();
                gparent.next[prela as usize] = NodeRef::none();
            } else {
                if gparent.flag.is_root() {
                    parent.flag.set_root();
                    new_root = Some(parent.clone());
                } else {
                    gparent.parent.set_child(parent.clone(), grela);
                    parent.flag.set_black();
                }
                gparent.next[crela as usize] = NodeRef::none();
                parent.set_child(gparent, toggle_rela(crela));
            }
        } else {
            uncle.into_owned().unwrap().flag.set_black();
            parent.flag.set_black();
            if gparent.flag.is_root() {
                gparent.flag.set_black();
            } else if gparent.parent.flag.is_red() {
                if let Some(nr) = gparent.rest_double_red_adjust() {
                    new_root = Some(nr);
                }
            }
        }
        new_root
    }
    pub(super) fn rasie(&mut self, rela: u8) -> Option<OwnedNodeRef<K, V>> {
        let mut new_root = None;
        let toggle_rela = toggle_rela(rela);
        let mut parent = self.clone();
        let mut brother = parent.next[toggle_rela as usize].get_owned();
        let prela = parent.flag.rela();
        if brother.flag.is_black() {
            let mut rnephew = brother.next[toggle_rela as usize].get_owned();
            let mut lnephew = brother.next[rela as usize].get_owned();
            if rnephew.flag.is_red() {
                if parent.flag.is_root() {
                    brother.flag.set_root();
                    new_root = Some(brother.clone());
                } else {
                    parent.parent.set_child(brother.clone(), prela);
                }
                brother.flag.set_color(parent.flag.color());
                parent.flag.set_black();
                rnephew.flag.set_black();
                parent.set_child(lnephew, toggle_rela);
                brother.set_child(parent, rela);
            } else if lnephew.flag.is_red() {
                if parent.flag.is_root() {
                    lnephew.flag.set_root();
                    new_root = Some(lnephew.clone());
                } else {
                    parent.parent.set_child(lnephew.clone(), prela);
                }
                lnephew.flag.set_color(parent.flag.color());
                parent.flag.set_black();
                parent.set_child(lnephew.next[rela as usize].get_owned(), toggle_rela);
                brother.set_child(lnephew.next[toggle_rela as usize].get_owned(), rela);
                lnephew.set_child(parent, rela);
                lnephew.set_child(brother, toggle_rela);
            } else {
                brother.flag.set_red();
                if parent.flag.is_red() {
                    parent.flag.set_black();
                } else if !parent.flag.is_root() {
                    if let Some(nr) = parent.parent.rasie(prela) {
                        new_root = Some(nr);
                    }
                }
            }
        } else {
            if parent.flag.is_root() {
                brother.flag.set_root();
                new_root = Some(brother.clone());
            } else {
                brother.flag.set_black();
                parent.parent.set_child(brother.clone(), prela);
            }
            parent.flag.set_red();
            parent.set_child(brother.next[rela as usize].get_owned(), toggle_rela);
            brother.set_child(parent.clone(), rela);

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
        unsafe { self.ptr.as_ref() }
    }
}
impl<K, V> DerefMut for OwnedNodeRef<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}
