use num_traits::{Num, NumAssignOps};
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;

#[derive(SerdeDiff, Serialize, Deserialize, Copy, Clone, Eq, PartialEq, Debug)]
pub struct Counter<N>
where
    N: SerdeDiff + Num + NumAssignOps + Clone,
{
    n: N,
}

#[allow(dead_code)]
impl<N: SerdeDiff> Counter<N>
where
    N: Num + NumAssignOps + Clone,
{
    pub fn new() -> Self {
        Self { n: N::zero() }
    }

    pub fn from(n: N) -> Self {
        Self { n }
    }

    pub fn get(&self) -> N {
        self.n.clone()
    }

    // Increment and get
    pub fn inc(&mut self) -> N {
        self.n += N::one();
        self.n.clone()
    }

    // Increment and get
    pub fn inc_and_get(&mut self) -> N {
        self.inc()
    }

    // Get value pre-increment, then increment internal value
    pub fn get_and_inc(&mut self) -> N {
        let n = self.n.clone();
        self.n += N::one();
        n
    }

    pub fn add(&mut self, n: N) -> N {
        self.n += n;
        self.n.clone()
    }
}

impl<N: SerdeDiff> From<N> for Counter<N>
where
    N: Num + NumAssignOps + Clone,
{
    fn from(value: N) -> Counter<N> {
        Counter::<N>::from(value)
    }
}
