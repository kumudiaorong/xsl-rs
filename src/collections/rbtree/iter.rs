use super::node::NodeRef;
use core::{
    iter::{FusedIterator, Iterator},
    marker::PhantomData,
};
enum LazyPoint<K, V> {
    Ready(NodeRef<K, V>),
    Moving(NodeRef<K, V>),
}
impl<K, V> Clone for LazyPoint<K, V> {
    fn clone(&self) -> Self {
        match self {
            LazyPoint::Ready(ptr) => LazyPoint::Ready(ptr.clone()),
            LazyPoint::Moving(ptr) => LazyPoint::Moving(ptr.clone()),
        }
    }
}

pub struct Iter<'a, K: 'a, V: 'a> {
    range: (LazyPoint<K, V>, LazyPoint<K, V>),
    length: usize,
    _marker: PhantomData<&'a (K, V)>,
}

impl<'a, K, V> Clone for Iter<'a, K, V> {
    fn clone(&self) -> Self {
        Self {
            range: self.range.clone(),
            length: self.length,
            _marker: PhantomData,
        }
    }
}

impl<'a, K, V> Iter<'a, K, V> {
    pub(super) fn new(root: NodeRef<K, V>, length: usize) -> Self {
        Self {
            range: (
                LazyPoint::Ready(root.clone()),
                LazyPoint::Ready(root.clone()),
            ),
            length,
            _marker: PhantomData,
        }
    }
    pub(super) fn new_empty() -> Self {
        Self {
            range: (
                LazyPoint::Ready(NodeRef::none()),
                LazyPoint::Ready(NodeRef::none()),
            ),
            length: 0,
            _marker: PhantomData,
        }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        if self.length == 0 {
            return None;
        }
        let new_begin = match self.range.0.clone() {
            LazyPoint::Ready(root) => root.min(),
            LazyPoint::Moving(begin) => begin.next_unchecked(),
        };
        self.range.0 = LazyPoint::Moving(new_begin.clone());
        self.length -= 1;
        let kv = &new_begin.into_ref().key_value;
        Some((&kv.0, &kv.1))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.length, Some(self.length))
    }
    fn last(self) -> Option<(&'a K, &'a V)> {
        if self.length == 0 {
            return None;
        }
        Some(
            {
                match self.range.1 {
                    LazyPoint::Ready(root) => root.max(),
                    LazyPoint::Moving(end) => end.next_back_unchecked(),
                }
            }
            .into_ref_key_value(),
        )
    }
    fn min(self) -> Option<(&'a K, &'a V)>
    where
        (&'a K, &'a V): Ord,
    {
        if self.length == 0 {
            return None;
        }
        Some(
            {
                match self.range.0 {
                    LazyPoint::Ready(root) => root.min(),
                    LazyPoint::Moving(begin) => begin.next_unchecked(),
                }
            }
            .into_ref_key_value(),
        )
    }
    fn max(self) -> Option<(&'a K, &'a V)>
    where
        (&'a K, &'a V): Ord,
    {
        self.last()
    }
}
impl<K, V> FusedIterator for Iter<'_, K, V> {}

impl<'a, K: 'a, V: 'a> DoubleEndedIterator for Iter<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
        if self.length == 0 {
            return None;
        }
        let new_end = match self.range.1.clone() {
            LazyPoint::Ready(root) => root.max(),
            LazyPoint::Moving(end) => end.next_back_unchecked(),
        };
        self.range.1 = LazyPoint::Moving(new_end.clone());
        self.length -= 1;
        Some(new_end.into_ref_key_value())
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {
    fn len(&self) -> usize {
        self.length
    }
}
pub struct IterMut<'a, K: 'a, V: 'a> {
    range: (LazyPoint<K, V>, LazyPoint<K, V>),
    length: usize,
    _marker: PhantomData<&'a mut (K, V)>,
}

impl<'a, K, V> IterMut<'a, K, V> {
    pub(super) fn new(root: NodeRef<K, V>, length: usize) -> Self {
        Self {
            range: (
                LazyPoint::Ready(root.clone()),
                LazyPoint::Ready(root.clone()),
            ),
            length,
            _marker: PhantomData,
        }
    }
    pub(super) fn new_empty() -> Self {
        Self {
            range: (
                LazyPoint::Ready(NodeRef::none()),
                LazyPoint::Ready(NodeRef::none()),
            ),
            length: 0,
            _marker: PhantomData,
        }
    }
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            range: (self.range.0.clone(), self.range.1.clone()),
            length: self.length,
            _marker: PhantomData,
        }
    }
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<Self::Item> {
        if self.length == 0 {
            return None;
        }
        let new_begin = match self.range.0.clone() {
            LazyPoint::Ready(root) => root.min(),
            LazyPoint::Moving(begin) => begin.next_unchecked(),
        };
        self.range.0 = LazyPoint::Moving(new_begin.clone());
        self.length -= 1;
        let kv = &mut new_begin.into_mut().key_value;
        Some((&kv.0, &mut kv.1))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.length, Some(self.length))
    }
    fn last(self) -> Option<(&'a K, &'a mut V)> {
        if self.length == 0 {
            return None;
        }
        let kv = &mut match self.range.1 {
            LazyPoint::Ready(root) => root.max(),
            LazyPoint::Moving(end) => end.next_back_unchecked(),
        }
        .into_mut()
        .key_value;
        Some((&kv.0, &mut kv.1))
    }
    fn min(self) -> Option<(&'a K, &'a mut V)>
    where
        (&'a K, &'a mut V): Ord,
    {
        if self.length == 0 {
            return None;
        }
        let kv = &mut match self.range.0 {
            LazyPoint::Ready(root) => root.min(),
            LazyPoint::Moving(begin) => begin.next_unchecked(),
        }
        .into_mut()
        .key_value;
        Some((&kv.0, &mut kv.1))
    }
    fn max(self) -> Option<(&'a K, &'a mut V)>
    where
        (&'a K, &'a mut V): Ord,
    {
        self.last()
    }
}
impl<'a, K, V> DoubleEndedIterator for IterMut<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
        if self.length == 0 {
            return None;
        }
        let new_end = match self.range.1.clone() {
            LazyPoint::Ready(root) => root.max(),
            LazyPoint::Moving(end) => end.next_back_unchecked(),
        };
        self.range.1 = LazyPoint::Moving(new_end.clone());
        self.length -= 1;
        let kv = &mut new_end.into_mut().key_value;
        Some((&kv.0, &mut kv.1))
    }
}
impl<K, V> ExactSizeIterator for IterMut<'_, K, V> {
    fn len(&self) -> usize {
        self.length
    }
}
