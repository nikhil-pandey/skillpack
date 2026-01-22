use color_eyre::eyre::{Result, eyre};
use color_eyre::Section as _;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct PatternSet {
    patterns: Vec<String>,
}

impl PatternSet {
    pub fn new(patterns: &[String]) -> Result<Self> {
        for pat in patterns {
            if !is_valid_pattern(pat) {
                return Err(eyre!("invalid pattern: {pat}")
                    .suggestion("Use * within segments and ** for any depth"));
            }
        }
        Ok(Self {
            patterns: patterns.to_vec(),
        })
    }

    pub fn is_match(&self, text: &str) -> bool {
        self.patterns.iter().any(|pat| match_pattern(pat, text))
    }

    pub fn match_all(&self, texts: &[String]) -> Vec<String> {
        texts
            .iter()
            .filter(|text| self.is_match(text))
            .cloned()
            .collect()
    }

    pub fn match_count_per_pattern(&self, texts: &[String]) -> Vec<usize> {
        self.patterns
            .iter()
            .map(|pat| texts.iter().filter(|t| match_pattern(pat, t)).count())
            .collect()
    }
}

pub fn is_valid_pattern(pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    pattern.split('/').all(|seg| {
        if seg.is_empty() {
            return false;
        }
        if seg.contains("**") && seg != "**" {
            return false;
        }
        true
    })
}

pub fn match_pattern(pattern: &str, text: &str) -> bool {
    let pat_segments: Vec<&str> = pattern.split('/').collect();
    let text_segments: Vec<&str> = text.split('/').collect();
    let mut memo = HashSet::new();
    match_segments(&pat_segments, &text_segments, 0, 0, &mut memo)
}

fn match_segments(
    pat: &[&str],
    text: &[&str],
    pi: usize,
    ti: usize,
    memo: &mut HashSet<(usize, usize)>,
) -> bool {
    if !memo.insert((pi, ti)) {
        return false;
    }
    if pi == pat.len() {
        return ti == text.len();
    }
    if pat[pi] == "**" {
        if match_segments(pat, text, pi + 1, ti, memo) {
            return true;
        }
        if ti < text.len() {
            return match_segments(pat, text, pi, ti + 1, memo);
        }
        return false;
    }
    if ti >= text.len() {
        return false;
    }
    if segment_match(pat[pi], text[ti]) {
        return match_segments(pat, text, pi + 1, ti + 1, memo);
    }
    false
}

fn segment_match(pattern: &str, text: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == text;
    }
    if pattern == "*" {
        return true;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0;
    if !pattern.starts_with('*') {
        let first = parts.first().unwrap_or(&"");
        if !text.starts_with(first) {
            return false;
        }
        pos = first.len();
    }
    if !pattern.ends_with('*') {
        let last = parts.last().unwrap_or(&"");
        if !text.ends_with(last) {
            return false;
        }
    }
    for (idx, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if idx == 0 && !pattern.starts_with('*') {
            continue;
        }
        if let Some(found) = text[pos..].find(part) {
            pos += found + part.len();
        } else {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::match_pattern;

    #[test]
    fn match_pattern_segments() {
        assert!(match_pattern("general/**", "general"));
        assert!(match_pattern("general/**", "general/foo"));
        assert!(match_pattern("coding/dotnet/*", "coding/dotnet/efcore"));
        assert!(!match_pattern("coding/dotnet/*", "coding/dotnet/efcore/x"));
        assert!(match_pattern("**/experimental/**", "experimental/foo"));
        assert!(match_pattern("**/experimental/**", "a/b/experimental/foo"));
        assert!(!match_pattern("**/experimental/**", "a/b/experiments/foo"));
    }

    #[test]
    fn match_pattern_segment_wildcards() {
        assert!(match_pattern("general/*-style", "general/writing-style"));
        assert!(match_pattern("general/*style", "general/writing-style"));
        assert!(!match_pattern("general/*style", "general/writing/ins"));
        assert!(!match_pattern("general/writing-style", "general/writing"));
    }
}
