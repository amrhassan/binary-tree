//! Counting trees.
//!
//! ## When should you use CountTree?
//!
//! - You want to maintain a possibly large unsorted list.
//! - You want to access, modify, insert, and delete elements at arbitrary
//!   position with O(logn) time complexity.
//! - You can tolerate O(n logn) time-complexity for:
//!   - splitting at arbitrary position
//!   - truncating the length
//!   - appending another list
//! - You have less than 4.29 billion elements!

use std::mem;
use std::iter::FromIterator;

use Node;
use NodeMut;
use BinaryTree;
use iter::Iter as GenIter;
use iter::IntoIter as GenIntoIter;

pub type NodePtr<T> = Box<CountNode<T>>;

pub struct CountTree<T>(Option<NodePtr<T>>);

impl<T> CountTree<T> {
    pub fn new() -> CountTree<T> {
        CountTree(None)
    }

    /// Returns the number elements in the tree. This is an O(1) operation.
    pub fn len(&self) -> usize {
        self.root().map_or(0, |node| node.count as usize)
    }

    /// Returns the element at the given index, or `None` if index is out of
    /// bounds. This is an O(log(n)) operation (worst case).
    pub fn get<'a>(&'a self, index: usize) -> Option<&'a T> {
        use WalkAction::*;

        if index >= self.len() {
            None
        } else {
            let mut val = None;
            let mut up_count = 0;
            self.root().unwrap().walk(|node: &'a CountNode<T>| {
                let cur_index = node.lcount() as usize + up_count;
                if index < cur_index {
                    Left
                } else if index == cur_index {
                    val = Some(node.value());
                    Stop
                } else {
                    up_count = cur_index + 1;
                    Right
                }
            });
            assert!(val.is_some());
            val
        }
    }

    // TODO get_mut

    /// Inserts a value at the given index. This is an O(log(n)) operation (worst case).
    ///
    /// # Panics
    ///
    /// Panics if index is greater than `self.len()`
    pub fn insert(&mut self, index: usize, value: T) {
        use WalkAction::*;

        let len = self.len();
        let new_node = Box::new(CountNode::new(value));
        if len == 0 && index == 0 {
            self.0 = Some(new_node);
        } else if index < len {
            let ref mut up_count = 0;
            self.0.as_mut().unwrap().walk_mut(move |node| {
                let cur_index = node.lcount() as usize + *up_count;
                if index < cur_index {
                    Left
                } else if index == cur_index {
                    Stop
                } else {
                    *up_count = cur_index + 1;
                    Right
                }
            }, move |node| {
                node.insert_before(new_node, |node, _| node.rebalance());
            }, |node, _| node.rebalance());
        } else if index == len {
            self.0.as_mut().unwrap().walk_mut(|_| Right,
                                              move |node| {
                                                  node.insert_right(Some(new_node));
                                              },
                                              |node, _| node.rebalance());
        } else {
            panic!("index out of bounds!");
        }
    }

    // TODO ? clear, is_empty, iter_mut
    // TODO { O(n) } truncate, append, split_off, retain
}

impl<T> BinaryTree for CountTree<T> {
    type Node = CountNode<T>;

    fn root(&self) -> Option<&Self::Node> {
        self.0.as_ref().map(|nodeptr| &**nodeptr)
    }
}

// prevent the unlikely event of stack overflow
impl<T> Drop for CountTree<T> {
    fn drop(&mut self) {
        let mut inner = None;
        mem::swap(&mut self.0, &mut inner);
        let _: GenIntoIter<CountNode<T>> = GenIntoIter::new(inner);
    }
}

fn is_power(v: u32) -> bool {
    if v == 0 {
        false
    } else {
        v & (v - 1) == 0
    }
}

fn exp_floor_log(v: u32) -> u32 {
    if v == 0 || is_power(v) {
        v
    } else {
        let mut efl = v - 1;
        efl |= efl >> 1;
        efl |= efl >> 2;
        efl |= efl >> 4;
        efl |= efl >> 8;
        efl |= efl >> 16;
        efl += 1;
        efl >> 1
    }
}

impl<T> FromIterator<T> for CountTree<T> {
    /// Creates a balanced binary tree in O(n + log^2(n)) time
    fn from_iter<I>(iterable: I) -> Self where I: IntoIterator<Item=T> {
        use WalkAction::*;

        let mut iter = iterable.into_iter();
        if let Some(item) = iter.next() {
            let mut node = Box::new(CountNode::new(item));
            let mut count = 1;
            while let Some(item) = iter.next() {
                let mut new_node = Box::new(CountNode::new(item));
                new_node.insert_left(Some(node));
                node = new_node;
                count += 1;
                let rcount = if is_power(count + 1) {
                    count >> 1
                } else {
                    count
                };
                let mut rotate_points = 1;
                while rcount & rotate_points == rotate_points {
                    node.rotate_right().unwrap();
                    rotate_points <<= 1;
                    rotate_points |= 1;
                }
            }
            let balanced_till = exp_floor_log(count + 1) - 1;
            count = node.lcount() + 1; // not needed
            while count > balanced_till {
                node.rotate_right().unwrap();
                node.right.as_mut().unwrap().walk_mut(|node| {
                    if node.balance_factor() > 1 {
                        node.rotate_right().unwrap();
                        Right
                    } else {
                        Stop
                    }
                }, |_| (), |_, _| ());
                count = node.lcount() + 1;
            }
            CountTree(Some(node))
        } else {
            CountTree::new()
        }
    }
}

impl<'a, T> IntoIterator for &'a CountTree<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            inner: GenIter::new(self.root()),
            remaining: self.len(),
        }
    }
}

pub struct Iter<'a, T: 'a> {
    inner: GenIter<'a, CountNode<T>>,
    remaining: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if self.remaining > 0 {
            self.remaining -= 1;
        }
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {}

impl<T> IntoIterator for CountTree<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(mut self) -> Self::IntoIter {
        let len = self.len();
        let mut inner = None;
        mem::swap(&mut self.0, &mut inner);
        IntoIter {
            inner: GenIntoIter::new(inner),
            remaining: len,
        }
    }
}

pub struct IntoIter<T> {
    inner: GenIntoIter<CountNode<T>>,
    remaining: usize,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.remaining > 0 {
            self.remaining -= 1;
        }
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {}

pub struct CountNode<T> {
    val: T,
    left: Option<NodePtr<T>>,
    right: Option<NodePtr<T>>,
    count: u32,
    height: u16,
}

impl<T> CountNode<T> {
    pub fn new(val: T) -> CountNode<T> {
        CountNode {
            val: val,
            left: None,
            right: None,
            count: 1,
            height: 0,
        }
    }

    fn lcount(&self) -> u32 {
        self.left.as_ref().map_or(0, |tree| tree.count)
    }

    fn rcount(&self) -> u32 {
        self.right.as_ref().map_or(0, |tree| tree.count)
    }

    // generalized version of AVL tree balance factor: h(left) - h(right)
    fn balance_factor(&self) -> i32 {
        if self.count == 1 {
            0
        } else if self.left.is_none() {
            -1 - self.right.as_ref().unwrap().height as i32
        } else if self.right.is_none() {
            1 + self.left.as_ref().unwrap().height as i32
        } else {
            self.left.as_ref().unwrap().height as i32 -
                self.right.as_ref().unwrap().height as i32
        }
    }

    // AVL tree algorithm
    fn rebalance(&mut self) {
        if self.balance_factor() > 1 {
            self.left.as_mut().map(|node| {
                if node.balance_factor() < 0 {
                    node.rotate_left().unwrap();
                }
            });
            self.rotate_right().unwrap();
        } else if self.balance_factor() < -1 {
            self.right.as_mut().map(|node| {
                if node.balance_factor() > 0 {
                    node.rotate_right().unwrap();
                }
            });
            self.rotate_left().unwrap();
        }
    }

    fn update_stats(&mut self) {
        use std::cmp::max;
        self.count = self.lcount() + self.rcount() + 1;
        self.height = max(self.left.as_ref().map_or(0, |tree| tree.height),
                          self.right.as_ref().map_or(0, |tree| tree.height));
        if self.count > 1 {
            self.height += 1;
        }
    }
}

impl<T> Node for CountNode<T> {
    type Value = T;

    fn left(&self) -> Option<&Self> {
        self.left.as_ref().map(|st| &**st)
    }

    fn right(&self) -> Option<&Self> {
        self.right.as_ref().map(|st| &**st)
    }

    fn value(&self) -> &T {
        &self.val
    }
}

impl<T> NodeMut for CountNode<T> {
    type NodePtr = NodePtr<T>;

    fn detach_left(&mut self) -> Option<Self::NodePtr> {
        let tree = self.left.take();
        self.update_stats();
        tree
    }

    fn detach_right(&mut self) -> Option<Self::NodePtr> {
        let tree = self.right.take();
        self.update_stats();
        tree
    }

    fn insert_left(&mut self, mut tree: Option<Self::NodePtr>) -> Option<Self::NodePtr> {
        mem::swap(&mut self.left, &mut tree);
        self.update_stats();
        tree
    }

    fn insert_right(&mut self, mut tree: Option<Self::NodePtr>) -> Option<Self::NodePtr> {
        mem::swap(&mut self.right, &mut tree);
        self.update_stats();
        tree
    }

    fn value_owned(self) -> T {
        self.val
    }
}

#[cfg(test)]
mod tests {
    use BinaryTree;
    use Node;
    use NodeMut;
    use super::CountNode;
    use super::CountTree;

    fn test_nodes() -> Box<CountNode<u32>> {
        let mut cn = Box::new(CountNode::new(7));
        cn.insert_before(Box::new(CountNode::new(8)), |_, _| ());
        cn.insert_before(Box::new(CountNode::new(12)), |_, _| ());
        cn.insert_right(Some(Box::new(CountNode::new(5))));
        cn
    }

    #[test]
    fn custom() {
        let ct = CountTree(Some(test_nodes()));
        assert_eq!(ct.get(0), Some(&8));
        assert_eq!(ct.get(1), Some(&12));
        assert_eq!(ct.get(2), Some(&7));
        assert_eq!(ct.get(3), Some(&5));
        assert_eq!(ct.get(4), None);
    }

    #[test]
    fn counting() {
        let cn = test_nodes();
        assert_eq!(cn.lcount(), 2);
        assert_eq!(cn.rcount(), 1);
        assert_eq!(cn.count, 4);
        assert_eq!(cn.height, 2);
    }

    #[test]
    fn rebalance() {
        let mut cn = test_nodes();
        assert_eq!(cn.balance_factor(), 1);
        cn.detach_right();
        cn.rebalance();
        assert_eq!(cn.balance_factor(), 0);
        let ct = CountTree(Some(cn));
        assert_eq!(ct.get(0), Some(&8));
        assert_eq!(ct.get(1), Some(&12));
        assert_eq!(ct.get(2), Some(&7));
        assert_eq!(ct.get(3), None);
    }

    #[test]
    fn insert() {
        let mut ct = CountTree::new();
        assert_eq!(ct.get(0), None);
        ct.insert(0, 1);
        ct.insert(0, 2);
        ct.insert(0, 3);
        ct.insert(0, 4);
        ct.insert(0, 5);
        ct.insert(0, 6);
        assert_eq!(ct.get(0), Some(&6));
        assert_eq!(ct.get(1), Some(&5));
        assert_eq!(ct.get(2), Some(&4));
        assert_eq!(ct.get(3), Some(&3));
        assert_eq!(ct.get(4), Some(&2));
        assert_eq!(ct.get(5), Some(&1));
        assert_eq!(ct.root().unwrap().balance_factor(), 0);
        assert_eq!(ct.root().unwrap().value(), &4);
        assert_eq!(ct.root().unwrap().left().unwrap().value(), &5);
        assert_eq!(ct.root().unwrap().right().unwrap().value(), &2);
        ct.insert(0, 7);
        assert_eq!(ct.root().unwrap().balance_factor(), 0);
        assert_eq!(ct.root().unwrap().left().unwrap().value(), &6);
        assert_eq!(ct.get(6), Some(&1));
        ct.insert(7, 0);
        assert_eq!(ct.root().unwrap().balance_factor(), -1);
        assert_eq!(ct.get(7), Some(&0));
    }

    #[test]
    fn from_iter() {
        let ct: CountTree<_> = (0..94).collect();
        let root = ct.root().unwrap();
        assert_eq!(root.value(), &31);
        assert_eq!(root.balance_factor(), -1);
        let left = root.left().unwrap();
        assert_eq!(left.value(), &15);
        assert_eq!(left.balance_factor(), 0);
        let right = root.right().unwrap();
        assert_eq!(right.value(), &63);
        assert_eq!(right.balance_factor(), 0);
        {
            let rl = right.left().unwrap();
            assert_eq!(rl.value(), &47);
            assert_eq!(rl.balance_factor(), 0);
            let rr = right.right().unwrap();
            assert_eq!(rr.value(), &79);
            assert_eq!(rr.balance_factor(), 0);
            {
                let rrl = rr.left().unwrap();
                assert_eq!(rrl.value(), &71);
                assert_eq!(rrl.balance_factor(), 0);
                let rrr = rr.right().unwrap();
                assert_eq!(rrr.value(), &87);
                assert_eq!(rrr.balance_factor(), 0);
                {
                    let rrrl = rrr.left().unwrap();
                    assert_eq!(rrrl.value(), &83);
                    assert_eq!(rrrl.balance_factor(), 0);
                    let rrrr = rrr.right().unwrap();
                    assert_eq!(rrrr.value(), &91);
                    assert_eq!(rrrr.balance_factor(), 0);
                    {
                        let rrrrl = rrrr.left().unwrap();
                        assert_eq!(rrrrl.value(), &89);
                        assert_eq!(rrrrl.balance_factor(), 0);
                        let rrrrr = rrrr.right().unwrap();
                        assert_eq!(rrrrr.value(), &93);
                        assert_eq!(rrrrr.balance_factor(), 1);
                        let rrrrrl = rrrrr.left().unwrap();
                        assert_eq!(rrrrrl.value(), &92);
                        assert_eq!(rrrrrl.balance_factor(), 0);
                    }
                }
            }
        }
    }
}
