use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PkgverToken {
    Numeric(String),
    Alpha(String),
}

pub fn compare_pkgver_strings(left: &str, right: &str) -> Ordering {
    let left_tokens = tokenize_pkgver(left);
    let right_tokens = tokenize_pkgver(right);
    let shared_len = left_tokens.len().min(right_tokens.len());

    for (left_token, right_token) in left_tokens.iter().zip(right_tokens.iter()) {
        let ordering = compare_token(left_token, right_token);
        if ordering != Ordering::Equal {
            return ordering;
        }
    }

    match left_tokens.len().cmp(&right_tokens.len()) {
        Ordering::Equal => Ordering::Equal,
        Ordering::Less => compare_missing_tail(&right_tokens[shared_len..]).reverse(),
        Ordering::Greater => compare_missing_tail(&left_tokens[shared_len..]),
    }
}

fn tokenize_pkgver(value: &str) -> Vec<PkgverToken> {
    let mut tokens = Vec::new();
    let mut chars = value.chars().peekable();

    while let Some(character) = chars.peek().copied() {
        if character.is_ascii_digit() {
            let mut numeric = String::new();
            while let Some(current) = chars.peek().copied() {
                if current.is_ascii_digit() {
                    numeric.push(current);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(PkgverToken::Numeric(numeric));
            continue;
        }

        if character.is_ascii_alphabetic() {
            let mut alpha = String::new();
            while let Some(current) = chars.peek().copied() {
                if current.is_ascii_alphabetic() {
                    alpha.push(current.to_ascii_lowercase());
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(PkgverToken::Alpha(alpha));
            continue;
        }

        chars.next();
    }

    tokens
}

fn compare_token(left: &PkgverToken, right: &PkgverToken) -> Ordering {
    match (left, right) {
        (PkgverToken::Numeric(left), PkgverToken::Numeric(right)) => {
            compare_numeric_runs(left, right)
        }
        (PkgverToken::Alpha(left), PkgverToken::Alpha(right)) => left.cmp(right),
        (PkgverToken::Numeric(_), PkgverToken::Alpha(_)) => Ordering::Greater,
        (PkgverToken::Alpha(_), PkgverToken::Numeric(_)) => Ordering::Less,
    }
}

fn compare_numeric_runs(left: &str, right: &str) -> Ordering {
    let left = left.trim_start_matches('0');
    let right = right.trim_start_matches('0');
    let left = if left.is_empty() { "0" } else { left };
    let right = if right.is_empty() { "0" } else { right };

    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn compare_missing_tail(remaining: &[PkgverToken]) -> Ordering {
    match remaining.first() {
        Some(PkgverToken::Alpha(_)) => Ordering::Less,
        Some(PkgverToken::Numeric(_)) => Ordering::Greater,
        None => Ordering::Equal,
    }
}
