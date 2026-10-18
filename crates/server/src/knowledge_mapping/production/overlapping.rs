//! Streaming exact byte KMP. Only the bounded quote has prefix storage; source
//! matches are yielded one at a time, including overlaps, and never collected.
struct Matches<'a> {
    source: &'a str,
    quote: &'a [u8],
    prefix: Vec<usize>,
    cursor: usize,
    matched: usize,
}

impl<'a> Matches<'a> {
    fn new(source: &'a str, quote: &'a str) -> Option<Self> {
        if quote.is_empty() || quote.len() > 4096 {
            return None;
        }
        let quote = quote.as_bytes();
        let mut prefix = vec![0; quote.len()];
        let mut matched = 0;
        for cursor in 1..quote.len() {
            while matched > 0 && quote[cursor] != quote[matched] {
                matched = prefix[matched - 1];
            }
            if quote[cursor] == quote[matched] {
                matched += 1;
            }
            prefix[cursor] = matched;
        }
        Some(Self {
            source,
            quote,
            prefix,
            cursor: 0,
            matched: 0,
        })
    }
}

impl Iterator for Matches<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        while self.cursor < self.source.len() {
            let byte = self.source.as_bytes()[self.cursor];
            while self.matched > 0 && byte != self.quote[self.matched] {
                self.matched = self.prefix[self.matched - 1];
            }
            if byte == self.quote[self.matched] {
                self.matched += 1;
            }
            self.cursor += 1;
            if self.matched == self.quote.len() {
                let start = self.cursor - self.quote.len();
                self.matched = self.prefix[self.matched - 1];
                if self.source.is_char_boundary(start) && self.source.is_char_boundary(self.cursor)
                {
                    return Some(start);
                }
            }
        }
        None
    }
}

pub(super) fn resolve(source: &str, quote: &str, occurrence: Option<usize>) -> Option<usize> {
    let mut matches = Matches::new(source, quote)?;
    match occurrence {
        Some(0) => None,
        Some(n) => matches.nth(n - 1),
        None => {
            let first = matches.next()?;
            matches.next().is_none().then_some(first)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(alphabet: &[char], max: usize) -> Vec<String> {
        let mut words = vec![String::new()];
        let mut level = vec![String::new()];
        for _ in 0..max {
            level = level
                .iter()
                .flat_map(|w| alphabet.iter().map(move |c| format!("{w}{c}")))
                .collect();
            words.extend(level.clone());
        }
        words
    }

    #[test]
    fn exhaustive_toys_match_independent_character_boundary_oracle() {
        for source in words(&['a', 'b', 'é'], 5) {
            for quote in words(&['a', 'b', 'é'], 3)
                .into_iter()
                .filter(|q| !q.is_empty())
            {
                let expected: Vec<_> = source
                    .char_indices()
                    .filter_map(|(i, _)| source[i..].starts_with(&quote).then_some(i))
                    .collect();
                assert_eq!(
                    Matches::new(&source, &quote).unwrap().collect::<Vec<_>>(),
                    expected
                );
                assert_eq!(
                    resolve(&source, &quote, None),
                    (expected.len() == 1).then(|| expected[0])
                );
                for n in 0..=expected.len() + 1 {
                    assert_eq!(
                        resolve(&source, &quote, Some(n)),
                        n.checked_sub(1).and_then(|i| expected.get(i).copied())
                    );
                }
                assert_eq!(resolve(&source, &quote, Some(usize::MAX)), None);
            }
        }
    }

    #[test]
    fn bounded_adversary_streams_and_stops_at_selected_match() {
        let source = "a".repeat(2 * 1024 * 1024);
        let quote = "a".repeat(4096);
        let mut matches = Matches::new(&source, &quote).unwrap();
        assert_eq!(matches.prefix.len(), 4096);
        assert_eq!(matches.next(), Some(0));
        assert_eq!(matches.cursor, 4096);
        assert_eq!(matches.next(), Some(1));
        assert_eq!(matches.cursor, 4097);
        assert_eq!(resolve(&source, &quote, None), None);
        assert_eq!(resolve(&source, &quote, Some(2)), Some(1));
        assert_eq!(
            resolve(&source, &format!("{}b", "a".repeat(4095)), None),
            None
        );
        assert_eq!(resolve(&source, &quote, Some(usize::MAX)), None);
        assert!(Matches::new(&source, &"a".repeat(4097)).is_none());
        assert!(Matches::new(&source, "").is_none());
    }
}
