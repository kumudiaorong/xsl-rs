use super::flag::{Flag, Rela};
use crate::alloc::{handle_alloc_error, Allocator};
use crate::ptr::Ptr;
use core::alloc::Layout;
use core::borrow::Borrow;
use core::fmt::Display;

fn comp<K>(l: &K, r: &K) -> Rela
where
    K: ?Sized + Ord,
{
    if l == r {
        Rela::PARENT
    } else if l < r {
        Rela::LEFT
    } else {
        Rela::RIGHT
    }
}
pub struct Node<K, V> {
    pub key_value: (K, V),
    pub next: [Ptr<Node<K, V>>; 3],
    pub flag: Flag,
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
    pub(super) fn new_in<A>(alloc: &A) -> Ptr<Self>
    where
        A: Allocator,
    {
        let layout = Layout::new::<Node<K, V>>();
        let ptr = match alloc.allocate_zeroed(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => handle_alloc_error(layout),
        };
        let ptr: Ptr<Self> = Ptr::new(ptr.cast());
        ptr
    }
    /// make a new node with key and value
    ///
    pub fn set_parent(&mut self, parent: Ptr<Self>, rela: Rela) {
        self.next[2] = parent;
        self.flag.set_rela(rela);
    }
    pub fn single_rotate(&mut self) {
        let mut parent_ptr = self.next[2];
        // let mut parent_node = parent_ptr.get_mut();
        let prela = parent_ptr.flag.rela();
        let mut gparent_ptr = parent_ptr.next[2];
        let child_ptr = Ptr::new(self as *mut _);
        let child_node = self;
        let crela = child_node.flag.rela();
        let mut gchild_ptr = child_node.next[crela.toggle() as usize];

        parent_ptr.next[crela as usize] = gchild_ptr;
        if !gchild_ptr.is_null() {
            gchild_ptr.set_parent(parent_ptr, crela);
        }
        // let test = unsafe { gparent_ptr.as_ref() };
        gparent_ptr.next[prela as usize] = child_ptr;
        parent_ptr.set_parent(child_ptr, crela.toggle());
        child_node.next[crela.toggle() as usize] = parent_ptr;
        child_node.set_parent(gparent_ptr, prela);
    }
    pub fn double_red_adjust(&mut self) {
        let mut child_node = self;
        let mut parent_ptr = child_node.next[2];
        let mut parent_node = parent_ptr.get_mut();
        let mut gparent_ptr = parent_node.next[2];
        let mut uncle_ptr = gparent_ptr.next[parent_node.flag.rela().toggle() as usize];
        gparent_ptr.flag.set_red();
        if uncle_ptr.is_null() || uncle_ptr.flag.is_black() {
            if child_node.flag.rela() != parent_node.flag.rela() {
                child_node.single_rotate();
                core::mem::swap(&mut child_node, &mut parent_node);
            }
            parent_node.single_rotate();
        } else {
            uncle_ptr.flag.set_black();
            let next_parent = gparent_ptr.next[2];
            if next_parent.is_null() {
                gparent_ptr.flag.set_black();
            } else if next_parent.flag.is_red() {
                gparent_ptr.double_red_adjust();
            } else if next_parent.flag.is_root() {
                gparent_ptr.flag.set_black();
            }
        }
        parent_node.flag.set_black();
    }
    pub(super) fn search<Q>(&self, key: &Q) -> (Ptr<Node<K, V>>, Result<(), Rela>)
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        let mut cur: Ptr<Self> = Ptr::new(self as *const _ as *mut _);
        let mut next;
        loop {
            let rela = comp(key, cur.key_value.0.borrow());
            if rela == Rela::PARENT {
                return (cur, Ok(()));
            }
            next = cur.next[rela as usize];
            if next.is_null() {
                return (cur, Err(Rela::from(rela)));
            }
            cur = next;
        }
    }
    pub(super) fn rasie(&mut self, rela: Rela) {
        let parent = self;
        let mut brother_ptr = parent.next[rela.toggle() as usize];
        if brother_ptr.flag.is_black() {
            let mut right_nephew_ptr = brother_ptr.next[rela.toggle() as usize];
            let mut left_nephew_ptr = brother_ptr.next[rela as usize];
            if right_nephew_ptr.flag.is_red() {
                brother_ptr.single_rotate();
                brother_ptr.flag.set_color(parent.flag.color());
                parent.flag.set_black();
                right_nephew_ptr.flag.set_black();
            } else if left_nephew_ptr.flag.is_red() {
                left_nephew_ptr.single_rotate();
                left_nephew_ptr.single_rotate();
                brother_ptr.flag.set_color(parent.flag.color());
                parent.flag.set_black();
            } else {
                brother_ptr.flag.set_red();
                if parent.flag.is_red() {
                    parent.flag.set_black();
                } else if parent.flag.rela() != Rela::PARENT {
                    parent.next[2].rasie(parent.flag.rela());
                }
            }
        } else {
            brother_ptr.single_rotate();
            parent.flag.set_red();
            brother_ptr.flag.set_black();
            parent.rasie(rela);
        }
    }
    pub(super) fn min(&self) -> Ptr<Self> {
        let mut cur = Ptr::new(self as *const _ as *mut _);
        let mut next = self.next[0];
        while !next.is_null() {
            cur = next;
            next = cur.next[0];
        }
        cur
    }
    pub(super) fn max(&self) -> Ptr<Self> {
        let mut cur = Ptr::new(self as *const _ as *mut _);
        let mut next = self.next[1];
        while !next.is_null() {
            cur = next;
            next = cur.next[1];
        }
        cur
    }
    pub(super) unsafe fn next_unchecked(&self) -> Ptr<Self> {
        if self.next[1].is_null() {
            if self.flag.rela() == Rela::LEFT {
                self.next[2]
            } else {
                let mut node = self.next[2].get();
                while node.flag.rela() == Rela::RIGHT {
                    node = node.next[2].get();
                }
                node.next[2]
            }
        } else {
            self.next[1].get().min()
        }
    }
    pub(crate) unsafe fn next_back_unchecked(&self) -> Ptr<Self> {
        if self.next[0].is_null() {
            if self.flag.rela() == Rela::RIGHT {
                self.next[2]
            } else {
                let mut node = self.next[2].get();
                while node.flag.rela() == Rela::LEFT {
                    node = node.next[2].get();
                }
                node.next[2]
            }
        } else {
            self.next[0].get().max()
        }
    }
}
