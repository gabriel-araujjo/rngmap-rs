use std::{fmt::Debug, borrow::Borrow};

use intrusive_collections::{intrusive_adapter, KeyAdapter, RBTreeLink, rbtree::CursorMut};

use crate::remove_until::RemoveUntil;


pub (crate) struct Node<K, V> {
    link: RBTreeLink,
    pub key: K,
    pub value: V,
}

impl<K, V> Node<K, V> {
    pub (crate) fn new(key: K, value: V) -> Box<Self> {
        Box::new(Self {
            link: RBTreeLink::default(),
            key,
            value,
        })
    }
}

impl<K, V> Debug for Node<K, V>
where
    K: Debug,
    V: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node {{ {:?} -> {:?} }}", self.key, self.value)
    }
}

intrusive_adapter!(pub (crate) NodeAdapter<K, V> = Box<Node<K, V>>: Node<K, V> { link: RBTreeLink });

impl<K, V> KeyAdapter<'_> for NodeAdapter<K, V>
where
    K: Copy,
{
    type Key = K;

    fn get_key(&self, node: &Node<K, V>) -> K {
        node.key.clone()
    }
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

            last_val = self.remove().map(|n| n.value);
        }
        last_val
    }
}
