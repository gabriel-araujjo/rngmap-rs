#![no_main]
use arbitrary::{Arbitrary, Unstructured, Error};
use libfuzzer_sys::fuzz_target;
use rngmap::RangeMap;
use std::ops::Bound;

#[derive(Debug)]
struct InsertArgs<K, V> {
    key: (Bound<K>, Bound<K>),
    value: V,
}

fn arbitrary_bound<'a, K: Arbitrary<'a>>(u: &mut Unstructured<'a>) -> arbitrary::Result<Bound<K>> {
    let kind = <u8 as Arbitrary<'a>>::arbitrary(u)?;

    if kind == 255 {
        Err(Error::IncorrectFormat)
    } else {
        match kind % 3 {
            0 => Ok(Bound::Included(K::arbitrary(u)?)),
            1 => Ok(Bound::Excluded(K::arbitrary(u)?)),
            2 => Ok(Bound::Unbounded),
            _ => unreachable!(),
        }
    }
}

impl<'a, K, V> Arbitrary<'a> for InsertArgs<K, V>
where
    K: Arbitrary<'a>,
    V: Arbitrary<'a>,
{
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            key: (arbitrary_bound(u)?, arbitrary_bound(u)?),
            value: V::arbitrary(u)?,
        })
    }
}

#[derive(Arbitrary, Debug)]
struct TestCase<K, V> {
    initial_value: V,
    inserts: Vec<InsertArgs<K, V>>,
}

fuzz_target!(|case: TestCase<i8, char>| {
    let TestCase {
        initial_value,
        inserts,
    } = case;
    let mut map = RangeMap::new(initial_value);

    map.check_canonical();

    for arg in inserts {
        let InsertArgs { key, value } = arg;
        map.insert(key, value);
        map.check_canonical();
    }
});
