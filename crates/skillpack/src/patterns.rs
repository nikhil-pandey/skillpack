use color_eyre::Section as _;
use color_eyre::eyre::{Result, eyre};
use globset::{Glob, GlobBuilder, GlobMatcher, GlobSet, GlobSetBuilder};

#[derive(Debug)]
pub struct PatternSet {
    matcher: GlobSet,
    per_pattern: Vec<PatternMatcher>,
}

impl PatternSet {
    pub fn new(patterns: &[String]) -> Result<Self> {
        let mut set_builder = GlobSetBuilder::new();
        let mut per_pattern = Vec::with_capacity(patterns.len());
        for pat in patterns {
            if !is_valid_pattern(pat) {
                return Err(eyre!("invalid pattern: {pat}")
                    .suggestion("Use * within segments and ** for any depth"));
            }
            let (matcher, globs) = build_matcher(pat)?;
            for glob in globs {
                set_builder.add(glob);
            }
            per_pattern.push(matcher);
        }
        let matcher = set_builder.build().map_err(|err| {
            eyre!("invalid pattern: {err}").suggestion("Use * within segments and ** for any depth")
        })?;
        Ok(Self {
            matcher,
            per_pattern,
        })
    }

    pub fn is_match(&self, text: &str) -> bool {
        self.matcher.is_match(text)
    }

    pub fn match_count_per_pattern(&self, texts: &[String]) -> Vec<usize> {
        self.per_pattern
            .iter()
            .map(|matcher| texts.iter().filter(|t| matcher.is_match(t)).count())
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
    if !is_valid_pattern(pattern) {
        return false;
    }
    build_matcher(pattern)
        .map(|(matcher, _)| matcher.is_match(text))
        .unwrap_or(false)
}

#[derive(Debug)]
struct PatternMatcher {
    primary: GlobMatcher,
    prefix: Option<GlobMatcher>,
}

impl PatternMatcher {
    fn is_match(&self, text: &str) -> bool {
        self.primary.is_match(text) || self.prefix.as_ref().is_some_and(|m| m.is_match(text))
    }
}

fn build_matcher(pattern: &str) -> Result<(PatternMatcher, Vec<Glob>)> {
    let primary_glob = build_glob(pattern)?;
    let primary = primary_glob.compile_matcher();
    let mut globs = vec![primary_glob];
    let prefix = if let Some(prefix_glob) = trailing_prefix_glob(pattern)? {
        let matcher = prefix_glob.compile_matcher();
        globs.push(prefix_glob);
        Some(matcher)
    } else {
        None
    };
    Ok((PatternMatcher { primary, prefix }, globs))
}

fn trailing_prefix_glob(pattern: &str) -> Result<Option<Glob>> {
    if !pattern.ends_with("/**") {
        return Ok(None);
    }
    let prefix = pattern.trim_end_matches("/**");
    if prefix.is_empty() {
        return Ok(None);
    }
    Ok(Some(build_glob(prefix)?))
}

fn build_glob(pattern: &str) -> Result<Glob> {
    let escaped = escape_pattern(pattern);
    GlobBuilder::new(&escaped)
        .literal_separator(true)
        .backslash_escape(true)
        .build()
        .map_err(|err| {
            eyre!("invalid pattern: {pattern}: {err}")
                .suggestion("Use * within segments and ** for any depth")
        })
}

fn escape_pattern(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len());
    for ch in pattern.chars() {
        match ch {
            '*' => out.push('*'),
            '?' | '[' | ']' | '{' | '}' | ',' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
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
