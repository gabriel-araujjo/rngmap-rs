use std::{
    borrow::Borrow,
    cell::Cell,
    cmp::{Ord, Ordering},
    fmt::Debug,
    ops::{Bound, Index, RangeBounds},
};

use intrusive_collections::{Adapter, intrusive_adapter, rbtree::CursorMut, Bound as ICBound, KeyAdapter, RBTree, RBTreeLink};
struct Node<K, V> {
    link: RBTreeLink,
    key: K,
    value: Cell<V>,
}

impl<K, V> Node<K, V> {
    fn new(key: K, value: V) -> Box<Self> {
        Box::new(Self {
            link: RBTreeLink::default(),
            key,
            value: Cell::new(value),
        })
    }

    fn value(&self) -> &V {
        // SAFETY: self owns node, so we can only change it with a &mut self
        // reference.
        unsafe { &*self.value.as_ptr() }
    }
}

impl<K, V> Debug for Node<K, V>
where
    K: Debug,
    V: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node {{ {:?} -> {:?} }}", self.key, self.value())
    }
}

intrusive_adapter!(NodeAdapter<K, V> = Box<Node<K, V>>: Node<K, V> { link: RBTreeLink });

impl<K, V> KeyAdapter<'_> for NodeAdapter<K, V>
where
    K: Copy,
{
    type Key = K;

    fn get_key(&self, node: &Node<K, V>) -> K {
        node.key.clone()
    }
}

#[derive(PartialEq, Eq, Ord, Clone, Copy)]
enum RangePoint<K> {
    Included(K),
    Excluded(K),
}

impl<K: PartialOrd> PartialOrd for RangePoint<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.point()
            .partial_cmp(other.point())
            .map(|ord| match ord {
                Ordering::Greater => Ordering::Greater,
                Ordering::Less => Ordering::Less,
                Ordering::Equal => match (self, other) {
                    (RangePoint::Included(_), RangePoint::Included(_))
                    | (RangePoint::Excluded(_), RangePoint::Excluded(_)) => Ordering::Equal,
                    (RangePoint::Included(_), RangePoint::Excluded(_)) => Ordering::Less,
                    (RangePoint::Excluded(_), RangePoint::Included(_)) => Ordering::Greater,
                },
            })
    }
}

impl<K> RangePoint<K> {
    fn from_ref_bound(bound: Bound<&K>) -> Option<Self>
    where
        K: Copy,
    {
        match bound {
            Bound::Included(k) => Some(Self::Included(*k)),
            Bound::Excluded(k) => Some(Self::Excluded(*k)),
            Bound::Unbounded => None,
        }
    }

    fn point(&self) -> &K {
        match self {
            RangePoint::Included(k) | RangePoint::Excluded(k) => k,
        }
    }

    fn swap_bound(self) -> Self {
        match self {
            RangePoint::Included(k) => RangePoint::Excluded(k),
            RangePoint::Excluded(k) => RangePoint::Included(k),
        }
    }
}

impl<K: Debug> Debug for RangePoint<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Included(k) => write!(f, "[{:?}", k),
            Self::Excluded(k) => write!(f, "{:?}]", k),
        }
    }
}

pub struct RangeMap<K, V> {
    val: V,
    map: RBTree<NodeAdapter<RangePoint<K>, V>>,
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

trait RemoveUntil<K> {
    type LastValue;
    fn remove_until(&mut self, upper_limit: Option<&K>) -> Option<Self::LastValue>;
}

impl<'a, K, Q, V> RemoveUntil<Q> for CursorMut<'a, NodeAdapter<K, V>>
where
    K: Borrow<Q>,
    Q: Ord,
{
    type LastValue = V;

    fn remove_until(&mut self, upper_limit: Option<&Q>) -> Option<Self::LastValue> {
        let mut last_val = None;

        while let Some(node) = self.get() {
            if let Some(upper_limit) = upper_limit {
                if node.key.borrow() > upper_limit {
                    break;
                }
            }

            last_val = self.remove().map(|n| n.value.into_inner());
        }
        last_val
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
        let start = RangePoint::from_ref_bound(range.start_bound());
        let end = RangePoint::from_ref_bound(range.end_bound());

        let mut cursor = match start {
            Some(ref start) => self.map.lower_bound_mut(ICBound::Included(start)),
            None => self.map.front_mut(),
        };

        let end_val = cursor
            .remove_until(end.as_ref())
            .unwrap_or(self.val.clone());

        if let Some(end) = end {
            let add_point = match cursor.get().map(|n| n.value().clone()) {
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
                .map(|n| n.value().clone())
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
        let bound = RangePoint::Included(index);
        match self.map.upper_bound(ICBound::Included(&bound)).get() {
            Some(node) => node.value(),
            None => &self.val,
        }
    }
}

#[test]
fn test_range_point_ord() {
    assert!(&RangePoint::Included(1) <= &RangePoint::Included(1));
    assert!(&RangePoint::Included(1) == &RangePoint::Included(1));
    assert!(&RangePoint::Included(1) >= &RangePoint::Included(1));

    assert!(&RangePoint::Excluded(1) > &RangePoint::Included(1));
    assert!(&RangePoint::Included(1) < &RangePoint::Excluded(1));

    assert!(&RangePoint::Included(-1) < &RangePoint::Excluded(0));

    assert!(&RangePoint::Excluded(1) != &RangePoint::Included(1));
    assert!(&RangePoint::Included(1) != &RangePoint::Excluded(1));

    assert!(&RangePoint::Excluded(-1) < &RangePoint::Included(0));
    assert!(&RangePoint::Included(-1) < &RangePoint::Included(0));
    assert!(&RangePoint::Excluded(-1) < &RangePoint::Excluded(0));
    assert!(&RangePoint::Included(-1) < &RangePoint::Excluded(0));
}

#[test]
fn test_range_point_swap_bound() {
    assert_eq!(
        RangePoint::Included(1).swap_bound(),
        RangePoint::Excluded(1)
    );
    assert_eq!(
        RangePoint::Excluded(1).swap_bound(),
        RangePoint::Included(1)
    );
}

#[test]
fn test_index() {
    let mut range = RangeMap::new('A');
    println!("{:?}", range);

    assert_eq!(range[10], 'A');

    range.insert(..10, 'B');
    println!("{:?}", range);

    assert_eq!(range[-10], 'B');
    assert_eq!(range[0], 'B');
    assert_eq!(range[10], 'A');
    assert_eq!(range[20], 'A');

    range.insert(..=-10, 'Z');
    println!("{:?}", range);

    assert_eq!(range[-10], 'Z');
    assert_eq!(range[0], 'B');
    assert_eq!(range[10], 'A');
    assert_eq!(range[20], 'A');

    range.insert((Bound::Excluded(10), Bound::Unbounded), 'C');
    println!("{:?}", range);

    assert_eq!(range[-10], 'Z');
    assert_eq!(range[0], 'B');
    assert_eq!(range[10], 'A');
    assert_eq!(range[20], 'C');
    assert_eq!(range[30], 'C');

    range.insert((Bound::Excluded(20), Bound::Unbounded), 'C');
    println!("{:?}", range);

    assert_eq!(range[-10], 'Z');
    assert_eq!(range[0], 'B');
    assert_eq!(range[10], 'A');
    assert_eq!(range[20], 'C');
    assert_eq!(range[30], 'C');

    range.insert(-15..-5, 'B');
    println!("{:?}", range);

    // range.insert(5..15, 'B');
    // println!("{:?}", range);

    // assert_eq!(range[-10], 'Z');
    // assert_eq!(range[0], 'B');
    // assert_eq!(range[10], 'A');
    // assert_eq!(range[20], 'C');
    // assert_eq!(range[30], 'C');
}
