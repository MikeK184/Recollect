//! Shared scalar ranking policy for qualified production and shadow labels.
//! Callers retain their exact scoped identities and qualify their own cohort.
use std::cmp::Reverse;

pub(crate) fn term_rank<K: Ord>(
    distinct_sources: usize,
    canonical_identity: K,
) -> (Reverse<usize>, K) {
    (Reverse(distinct_sources), canonical_identity)
}
