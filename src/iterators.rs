use std::ops::Bound;

use crate::{
    bound::Point,
    node::{Node, NodeAdapter},
    SectionMap,
};

type RBIter<'a, K, V> = intrusive_collections::rbtree::Iter<'a, NodeAdapter<Point<K>, V>>;

pub struct Iter<'a, K, V> {
    next: Option<(Bound<&'a K>, &'a V)>,
    iter: RBIter<'a, K, V>,
}

/// An iterator over the sections of a [`SectionMap`]
///
/// Each section is represented by a tuple with references to its bounds and its respective value,
/// or in other words <code>Self::Item = (([`Bound<&K>`], [`Bound<&K>`]), `&V`)</code>.
///
/// # Example
/// 
/// ```
/// # use std::ops::Bound;
/// # use rngmap::SectionMap;
/// #
/// let mut map = SectionMap::new("outer");
/// 
/// map.insert(-1..1, "inner");
/// 
/// let mut iter = map.iter();
/// 
/// let (section, value) = iter.next().unwrap();
///
/// assert_eq!(section, (Bound::Unbounded, Bound::Excluded(&-1)));
/// assert_eq!(*value, "outer");
/// 
/// let (section, value) = iter.next().unwrap();
/// 
/// assert_eq!(section, (Bound::Included(&-1), Bound::Excluded(&1)));
/// assert_eq!(*value, "inner");
/// 
/// let (section, value) = iter.next().unwrap();
///
/// assert_eq!(section, (Bound::Included(&1), Bound::Unbounded));
/// assert_eq!(*value, "outer");
/// 
/// assert_eq!(iter.next(), None);
/// ```
impl<'a, K, V> Iterator for Iter<'a, K, V> {
    /// Each section in a [`SectionMap`]
    type Item = ((Bound<&'a K>, Bound<&'a K>), &'a V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let next = std::mem::replace(&mut self.next, self.iter.next().map(Node::kv_ref));

        next.map(|(start, value)| {
            (
                (
                    start,
                    match self.next {
                        Some((Bound::Included(k), _)) => Bound::Excluded(k),
                        Some((Bound::Excluded(k), _)) => Bound::Included(k),
                        _ => Bound::Unbounded,
                    },
                ),
                value,
            )
        })
    }
}

impl<K, V> SectionMap<K, V> {
    pub fn iter(&self) -> Iter<K, V> {
        Iter {
            next: Some((Bound::Unbounded, &self.val)),
            iter: self.map.iter(),
        }
    }
}

impl<K, V> Node<Point<K>, V> {
    #[inline]
    fn kv_ref(&self) -> (Bound<&K>, &V) {
        (
            match self.key {
                Point::Included(ref k) => Bound::Included(k),
                Point::Excluded(ref k) => Bound::Excluded(k),
            },
            &self.value,
        )
    }
}
