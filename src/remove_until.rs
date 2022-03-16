pub (crate) trait RemoveUntil<K> {
    type LastValue;
    fn remove_until(&mut self, upper_limit: Option<&K>) -> Option<Self::LastValue>;
}
