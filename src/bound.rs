use std::{
    cmp::Ordering,
    fmt::Debug,
    ops::{Bound, RangeBounds},
};

#[derive(PartialEq, Eq, Ord, Clone, Copy)]
pub(crate) enum Point<K> {
    Included(K),
    Excluded(K),
}

impl<K: PartialOrd> PartialOrd for Point<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.inner()
            .partial_cmp(other.inner())
            .map(|ord| match ord {
                Ordering::Greater => Ordering::Greater,
                Ordering::Less => Ordering::Less,
                Ordering::Equal => match (self, other) {
                    (Point::Included(_), Point::Included(_))
                    | (Point::Excluded(_), Point::Excluded(_)) => Ordering::Equal,
                    (Point::Included(_), Point::Excluded(_)) => Ordering::Less,
                    (Point::Excluded(_), Point::Included(_)) => Ordering::Greater,
                },
            })
    }
}

impl<K> Point<K> {
    pub(crate) fn from_bound_ref(bound: Bound<&K>) -> Option<Self>
    where
        K: Copy,
    {
        match bound {
            Bound::Included(k) => Some(Self::Included(*k)),
            Bound::Excluded(k) => Some(Self::Excluded(*k)),
            Bound::Unbounded => None,
        }
    }

    fn inner(&self) -> &K {
        match self {
            Point::Included(k) | Point::Excluded(k) => k,
        }
    }

    pub(crate) fn swap_bound(self) -> Self {
        match self {
            Point::Included(k) => Point::Excluded(k),
            Point::Excluded(k) => Point::Included(k),
        }
    }
}

pub(crate) fn is_valid_range<R, K>(range: &R) -> bool
where
    R: RangeBounds<K>,
    K: Ord,
{
    let start = range.start_bound();
    let end = range.end_bound();

    match (start, end) {
        (Bound::Included(start), Bound::Included(end)) if start > end => false,
        (Bound::Included(start), Bound::Excluded(end))
        | (Bound::Excluded(start), Bound::Included(end))
        | (Bound::Excluded(start), Bound::Excluded(end))
            if start >= end =>
        {
            false
        }
        _ => true,
    }
}

impl<K: Debug> Debug for Point<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Included(k) => write!(f, "[{:?}", k),
            Self::Excluded(k) => write!(f, "{:?}]", k),
        }
    }
}

#[test]
fn test_point_ord() {
    assert!(&Point::Included(1) <= &Point::Included(1));
    assert!(&Point::Included(1) == &Point::Included(1));
    assert!(&Point::Included(1) >= &Point::Included(1));

    assert!(&Point::Excluded(1) > &Point::Included(1));
    assert!(&Point::Included(1) < &Point::Excluded(1));

    assert!(&Point::Included(-1) < &Point::Excluded(0));

    assert!(&Point::Excluded(1) != &Point::Included(1));
    assert!(&Point::Included(1) != &Point::Excluded(1));

    assert!(&Point::Excluded(-1) < &Point::Included(0));
    assert!(&Point::Included(-1) < &Point::Included(0));
    assert!(&Point::Excluded(-1) < &Point::Excluded(0));
    assert!(&Point::Included(-1) < &Point::Excluded(0));
}

#[test]
fn test_range_point_swap_bound() {
    assert_eq!(Point::Included(1).swap_bound(), Point::Excluded(1));
    assert_eq!(Point::Excluded(1).swap_bound(), Point::Included(1));
}

#[test]
fn test_is_valid_range() {
    assert!(is_valid_range::<_, i32>(&(..)));
    assert!(is_valid_range(&(..2)));
    assert!(is_valid_range(&(2..)));
    assert!(is_valid_range(&(0..1)));
    assert!(is_valid_range(&(0..=0))); // inc inc
    assert!(!is_valid_range(&(0..0))); // inc exc
    assert!(!is_valid_range(&(Bound::Excluded(0), Bound::Included(0)))); // exc inc
    assert!(!is_valid_range(&(Bound::Excluded(0), Bound::Excluded(0)))); // exc exc
    assert!(!is_valid_range(&(1..0)));
}
