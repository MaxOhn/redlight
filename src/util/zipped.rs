pub(crate) struct ZippedVecs<L, R> {
    left: Vec<L>,
    right: Vec<R>,
}

impl<L, R> ZippedVecs<L, R> {
    pub(crate) fn unzip(self) -> (Vec<L>, Vec<R>) {
        let Self { left, right } = self;

        (left, right)
    }
}

impl<L, R> FromIterator<(L, R)> for ZippedVecs<L, R> {
    fn from_iter<T: IntoIterator<Item = (L, R)>>(iter: T) -> Self {
        let mut vecs = (Vec::new(), Vec::new());
        vecs.extend(iter);
        let (left, right) = vecs;

        Self { left, right }
    }
}
