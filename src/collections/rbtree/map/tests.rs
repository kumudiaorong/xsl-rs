use crate::alloc::Allocator;
impl<K, V, A> super::RBTreeMap<K, V, A>
where
    K: Ord,
    A: Allocator + Clone,
{
    pub fn check(&self) {
        if self.is_empty() {
            return;
        }
        assert!(self.root.next[2].flag.is_black(), "root is not black");
        use crate::alloc::Vec;
        let mut max_height = 0;
        let mut stack = Vec::new();
        let mut end_flag = false;
        stack.push((self.root.next[2], 0));
        while let Some((node, mut height)) = stack.pop() {
            if node.is_null() {
                if end_flag {
                    assert!(height == max_height, "wrong height");
                } else {
                    end_flag = true;
                    max_height = height;
                }
                continue;
            }
            if node.flag.is_black() {
                height += 1;
            } else {
                assert!(
                    node.next[0].is_null() || node.next[0].flag.is_black(),
                    "red left child",
                );
                assert!(
                    node.next[1].is_null() || node.next[1].flag.is_black(),
                    "red right child",
                );
            }
            assert!(node.key_value.0 > node.next[0].key_value.0, "wrong order");
            assert!(node.key_value.0 < node.next[1].key_value.0, "wrong order");
            stack.push((node.next[1], height));
            stack.push((node.next[0], height));
        }
    }
}