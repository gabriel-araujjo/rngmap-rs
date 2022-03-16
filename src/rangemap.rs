use std::{
    cmp::Ord,
    fmt::Debug,
    ops::{Index, RangeBounds},
};

use intrusive_collections::{Bound as ICBound, RBTree};

use crate::bound::{is_valid_range, Point};
use crate::node::{Node, NodeAdapter};
use crate::remove_until::RemoveUntil;

pub struct RangeMap<K, V> {
    val: V,
    map: RBTree<NodeAdapter<Point<K>, V>>,
}

impl<K: Debug, V: Debug> Debug for RangeMap<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set()
            .entry(&self.val)
            .entries(self.map.iter())
            .finish()
    }
}

impl<K, V> RangeMap<K, V> {
    pub fn new(val: V) -> Self {
        Self {
            val,
            map: RBTree::default(),
        }
    }
}

impl<K, V: Default> Default for RangeMap<K, V> {
    fn default() -> Self {
        Self {
            val: Default::default(),
            map: RBTree::default(),
        }
    }
}

impl<K, V> RangeMap<K, V>
where
    K: Ord + Copy + Debug,
    V: Clone + Eq + Debug,
{
    pub fn insert<R>(&mut self, range: R, value: V)
    where
        R: RangeBounds<K>,
    {
        let start = Point::from_bound_ref(range.start_bound());
        let end = Point::from_bound_ref(range.end_bound());

        if !is_valid_range(&range) {
            return;
        }

        let mut cursor = match start {
            Some(ref start) => self.map.lower_bound_mut(ICBound::Included(start)),
            None => self.map.front_mut(),
        };

        // println!(
        //     "cursor = {:?}, end = {:?}",
        //     cursor.get(),
        //     end.map(Point::swap_bound)
        // );

        let end_val = cursor
            .remove_until(end.map(Point::swap_bound).as_ref())
            .unwrap_or_else(|| {
                cursor
                    .peek_prev()
                    .get()
                    .map(|n| n.value.clone())
                    .unwrap_or_else(|| self.val.clone())
            });

        if let Some(end) = end {
            let add_point = match cursor.get().map(|n| n.value.clone()) {
                Some(next_val) => value != end_val && end_val != next_val,
                None => value != end_val,
            };

            if add_point {
                cursor.insert_before(Node::new(end.swap_bound(), end_val));
                cursor.move_prev();
            }
        }

        if let Some(start) = start {
            let prev_val = cursor
                .peek_prev()
                .get()
                .map(|n| n.value.clone())
                .unwrap_or(self.val.clone());

            if value != prev_val {
                cursor.insert_before(Node::new(start, value));
            }
        } else {
            self.val = value;
        }
    }
}

impl<K, V> Index<K> for RangeMap<K, V>
where
    K: Ord + Copy,
{
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        let bound = Point::Included(index);
        match self.map.upper_bound(ICBound::Included(&bound)).get() {
            Some(node) => &node.value,
            None => &self.val,
        }
    }
}

#[cfg(any(fuzzing, test))]
impl<K, V> RangeMap<K, V>
where
    K: Ord + Debug,
    V: Ord + Debug,
{
    pub fn check_canonical(&self) {
        let mut last_key: Option<&Point<K>> = None;
        let mut last_value = &self.val;

        for node in self.map.iter() {
            if node.value == *last_value {
                panic!("repeated value: {:?}", last_value);
            }

            if let Some(last_key) = last_key {
                if !(*last_key < node.key) {
                    panic!(
                        "wrong key order: {:?} is not less then {:?}",
                        *last_key, node.key,
                    );
                }
            }

            last_key = Some(&node.key);
            last_value = &node.value;
        }
    }
}

#[cfg(test)]
mod test {

    use super::RangeMap;

    #[test]
    fn index() {
        use std::ops::Bound;

        let mut range = RangeMap::new('A');
        // println!("{:?}", range);

        assert_eq!(range[10], 'A');

        range.insert(..10, 'B');
        // println!("{:?}", range);

        assert_eq!(range[-10], 'B');
        assert_eq!(range[0], 'B');
        assert_eq!(range[10], 'A');
        assert_eq!(range[20], 'A');

        range.insert(..=-10, 'Z');
        // println!("{:?}", range);

        assert_eq!(range[-10], 'Z');
        assert_eq!(range[0], 'B');
        assert_eq!(range[10], 'A');
        assert_eq!(range[20], 'A');

        range.insert((Bound::Excluded(10), Bound::Unbounded), 'C');
        // println!("{:?}", range);

        assert_eq!(range[-10], 'Z');
        assert_eq!(range[0], 'B');
        assert_eq!(range[10], 'A');
        assert_eq!(range[20], 'C');
        assert_eq!(range[30], 'C');

        range.insert((Bound::Excluded(20), Bound::Unbounded), 'C');
        // println!("{:?}", range);

        assert_eq!(range[-10], 'Z');
        assert_eq!(range[0], 'B');
        assert_eq!(range[10], 'A');
        assert_eq!(range[20], 'C');
        assert_eq!(range[30], 'C');

        range.insert(-15..-5, 'B');
        // println!("{:?}", range);

        // range.insert(5..15, 'B');
        // println!("{:?}", range);

        // assert_eq!(range[-10], 'Z');
        // assert_eq!(range[0], 'B');
        // assert_eq!(range[10], 'A');
        // assert_eq!(range[20], 'C');
        // assert_eq!(range[30], 'C');
    }

    #[test]
    fn fuzzy_crash_20220315_1() {
        let mut range = RangeMap::new('Z');
        range.check_canonical();

        range.insert(-1..=16, 'B');
        range.check_canonical();

        range.insert(-1..=0, 'C');
        range.check_canonical();

        range.insert(0..=0, 'C');
        range.check_canonical();
    }

    #[test]
    fn fuzzy_crash_20220315_2() {
        let mut range = RangeMap::new('Z');
        assert_eq!(range[0], 'Z');
        range.check_canonical();

        range.insert(-1..=39, 'B');
        assert_eq!(range[-2], 'Z');
        assert_eq!(range[-1], 'B');
        assert_eq!(range[39], 'B');
        assert_eq!(range[40], 'Z');
        range.check_canonical();

        range.insert(0..=0, 'Z');
        assert_eq!(range[-2], 'Z');
        assert_eq!(range[-1], 'B');
        assert_eq!(range[0], 'Z');
        assert_eq!(range[1], 'B');
        assert_eq!(range[39], 'B');
        assert_eq!(range[40], 'Z');
        range.check_canonical();

        range.insert(0..=0, 'Z');
        assert_eq!(range[-2], 'Z');
        assert_eq!(range[-1], 'B');
        assert_eq!(range[0], 'Z');
        assert_eq!(range[1], 'B');
        assert_eq!(range[39], 'B');
        assert_eq!(range[40], 'Z');
        range.check_canonical();
    }
}
