use compiler_base::{
    unicode_properties::UnicodeEmoji,
    utility::uni_lexer::{Cursor, EOF_CHAR, is_rust_id_start, is_xid_continue},
};
use konst::const_panic::PanicFmt;

use crate::{Base, Comment, DocStyle, GuardedStr, LiteralKind, SpecialChar, TokenKind};

pub(crate) fn parse(
    cursor: &mut Cursor<'_, TokenKind>,
    prev_pos: usize,
    first_char: char,
) -> Option<TokenKind> {
    match first_char {
        '/' => match cursor.first() {
            '/' => Some(TokenKind::Comment(line_comment(cursor))),
            '*' => Some(TokenKind::Comment(block_comment(cursor))),
            _ => Some(TokenKind::SpecialChar(SpecialChar::Solidus)),
        },

        // Raw identifier, raw string literal or identifier.
        'r' => match (cursor.first(), cursor.second()) {
            ('#', c1) if is_rust_id_start(c1) => Some(raw_identifier(cursor)),
            ('#', _) | ('"', _) => {
                let res = raw_double_quoted_string(cursor, 1);
                let suffix_start = cursor.pos_within_token();
                if res.is_ok() {
                    eat_literal_suffix(cursor);
                }
                let kind = LiteralKind::RawStr { n_hashes: res.ok() };
                Some(TokenKind::Literal { kind, suffix_start })
            }
            _ => Some(identifier_or_unknown_prefix(cursor)),
        },

        // Byte literal, byte string literal, raw byte string literal or identifier.
        'b' => Some(byte_string(
            cursor,
            |terminated| LiteralKind::ByteStr { terminated },
            |n_hashes| LiteralKind::RawByteStr { n_hashes },
            Some(|terminated| LiteralKind::Byte { terminated }),
        )),

        // Identifier (this should be checked after other variant that can
        // start as identifier).
        c if is_rust_id_start(c) => Some(identifier_or_unknown_prefix(cursor)),

        // Numeric literal.
        c @ '0'..='9' => {
            let literal_kind = number(cursor, c);
            let suffix_start = cursor.pos_within_token();
            eat_literal_suffix(cursor);
            Some(TokenKind::Literal {
                kind: literal_kind,
                suffix_start,
            })
        }

        // Guarded string literal prefix: `#"` or `##`
        '#' if matches!(cursor.first(), '"' | '#') => {
            cursor.bump();
            Some(TokenKind::GuardedStrPrefix)
        }

        // Character literal.
        '\'' => Some(char(cursor)),

        '"' => {
            let terminated = double_quoted_string(cursor);
            let suffix_start = cursor.pos_within_token();
            if terminated {
                eat_literal_suffix(cursor);
            }
            let kind = LiteralKind::Str { terminated };
            Some(TokenKind::Literal { kind, suffix_start })
        }

        _ if let Some(special_char) =
            SpecialChar::parse_prefix(&cursor.original_source()[prev_pos..]) =>
        {
            cursor.bump_bytes(special_char.as_str().len() - 1);
            Some(TokenKind::SpecialChar(special_char))
        }

        c if !c.is_ascii() && c.is_emoji_char() => Some(invalid_identifier(cursor)),

        _ => None,
    }
}

/* #region Comment */

fn line_comment(cursor: &mut Cursor<'_, TokenKind>) -> Comment {
    debug_assert!(cursor.prev() == '/' && cursor.first() == '/');
    cursor.bump();

    let doc_style = match cursor.first() {
        // `//!` is an inner line doc comment.
        '!' => Some(DocStyle::Inner),
        // `////` (more than 3 slashes) is not considered a doc comment.
        '/' if cursor.second() != '/' => Some(DocStyle::Outer),
        _ => None,
    };

    cursor.eat_until(b'\n');
    Comment::Line(doc_style)
}

fn block_comment(cursor: &mut Cursor<'_, TokenKind>) -> Comment {
    debug_assert!(cursor.prev() == '/' && cursor.first() == '*');
    cursor.bump();

    let doc_style = match cursor.first() {
        // `/*!` is an inner block doc comment.
        '!' => Some(DocStyle::Inner),
        // `/***` (more than 2 stars) is not considered a doc comment.
        // `/**/` is not considered a doc comment.
        '*' if !matches!(cursor.second(), '*' | '/') => Some(DocStyle::Outer),
        _ => None,
    };

    let mut depth = 1usize;
    while let Some(c) = cursor.bump() {
        match c {
            '/' if cursor.first() == '*' => {
                cursor.bump();
                depth += 1;
            }
            '*' if cursor.first() == '/' => {
                cursor.bump();
                depth -= 1;
                if depth == 0 {
                    // This block comment is closed, so for a construction like "/* */ */"
                    // there will be a successfully parsed block comment "/* */"
                    // and " */" will be processed separately.
                    break;
                }
            }
            _ => (),
        }
    }

    Comment::Block {
        style: doc_style,
        terminated: depth == 0,
    }
}

/* #endregion */

/* #region Identifier */

fn raw_identifier(cursor: &mut Cursor<'_, TokenKind>) -> TokenKind {
    debug_assert!(
        cursor.prev() == 'r' && cursor.first() == '#' && is_rust_id_start(cursor.second())
    );
    // Eat "#" symbol.
    cursor.bump();
    // Eat the identifier part of RawIdent.
    eat_identifier(cursor);
    TokenKind::RawIdentifier
}

fn identifier_or_unknown_prefix(cursor: &mut Cursor<'_, TokenKind>) -> TokenKind {
    debug_assert!(is_rust_id_start(cursor.prev()));
    // Start is already eaten, eat the rest of identifier.
    cursor.eat_while(is_xid_continue);
    // Known prefixes must have been handled earlier. So if
    // we see a prefix here, it is definitely an unknown prefix.
    match cursor.first() {
        '#' | '"' | '\'' => TokenKind::UnknownPrefix,
        c if !c.is_ascii() && c.is_emoji_char() => invalid_identifier(cursor),
        _ => TokenKind::Identifier,
    }
}

fn invalid_identifier(cursor: &mut Cursor<'_, TokenKind>) -> TokenKind {
    // Start is already eaten, eat the rest of identifier.
    cursor.eat_while(|c| {
        const ZERO_WIDTH_JOINER: char = '\u{200d}';
        is_xid_continue(c) || (!c.is_ascii() && c.is_emoji_char()) || c == ZERO_WIDTH_JOINER
    });
    // An invalid identifier followed by '#' or '"' or '\'' could be
    // interpreted as an invalid literal prefix. We don't bother doing that
    // because the treatment of invalid identifiers and invalid prefixes
    // would be the same.
    TokenKind::InvalidIdentifier
}

/// Eats the identifier. Note: succeeds on `_`, which isn't a valid
/// identifier.
fn eat_identifier(cursor: &mut Cursor<'_, TokenKind>) {
    if !is_rust_id_start(cursor.first()) {
        return;
    }
    cursor.bump();

    cursor.eat_while(is_xid_continue);
}

/* #endregion */

/* #region Char */

fn char(cursor: &mut Cursor<'_, TokenKind>) -> TokenKind {
    debug_assert!(cursor.prev() == '\'');

    let terminated = single_quoted_string(cursor);
    let suffix_start = cursor.pos_within_token();
    if terminated {
        eat_literal_suffix(cursor);
    }
    let kind = LiteralKind::Char { terminated };
    TokenKind::Literal { kind, suffix_start }
}

fn single_quoted_string(cursor: &mut Cursor<'_, TokenKind>) -> bool {
    debug_assert!(cursor.prev() == '\'');
    // Check if it's a one-symbol literal.
    if cursor.second() == '\'' && cursor.first() != '\\' {
        cursor.bump();
        cursor.bump();
        return true;
    }

    // Literal has more than one symbol.

    // Parse until either quotes are terminated or error is detected.
    loop {
        match cursor.first() {
            // Quotes are terminated, finish parsing.
            '\'' => {
                cursor.bump();
                return true;
            }
            // Probably beginning of the comment, which we don't want to include
            // to the error report.
            '/' => break,
            // Newline without following '\'' means unclosed quote, stop parsing.
            '\n' if cursor.second() != '\'' => break,
            // End of file, stop parsing.
            #[allow(unused_variables)]
            EOF_CHAR if cursor.is_eof() => break,
            // Escaped slash is considered one character, so bump twice.
            '\\' => {
                cursor.bump();
                cursor.bump();
            }
            // Skip the character.
            _ => {
                cursor.bump();
            }
        }
    }
    // String was not terminated.
    false
}

/* #endregion */

/* #region String */

/// Eats double-quoted string and returns true
/// if string is terminated.
fn double_quoted_string(cursor: &mut Cursor<'_, TokenKind>) -> bool {
    debug_assert!(cursor.prev() == '"');
    while let Some(c) = cursor.bump() {
        match c {
            '"' => {
                return true;
            }
            '\\' if cursor.first() == '\\' || cursor.first() == '"' => {
                // Bump again to skip escaped character.
                cursor.bump();
            }
            _ => (),
        }
    }
    // End of file reached.
    false
}

/// Attempt to lex for a guarded string literal.
///
/// Used by `rustc_parse::lexer` to lex for guarded strings
/// conditionally based on edition.
///
/// Note: this will not reset the `Cursor` when a
/// guarded string is not found. It is the caller's
/// responsibility to do so.
#[allow(unused)]
pub fn guarded_double_quoted_string(cursor: &mut Cursor<'_, TokenKind>) -> Option<GuardedStr> {
    debug_assert!(cursor.prev() != '#');

    let mut n_start_hashes: u32 = 0;
    while cursor.first() == '#' {
        n_start_hashes += 1;
        cursor.bump();
    }

    if cursor.first() != '"' {
        return None;
    }
    cursor.bump();
    debug_assert!(cursor.prev() == '"');

    // Lex the string itself as a normal string literal
    // so we can recover that for older editions later.
    let terminated = double_quoted_string(cursor);
    if !terminated {
        let token_len = cursor.pos_within_token();
        cursor.reset_pos_within_token();

        return Some(GuardedStr {
            n_hashes: n_start_hashes,
            terminated: false,
            token_len,
        });
    }

    // Consume closing '#' symbols.
    // Note that this will not consume extra trailing `#` characters:
    // `###"abcde"####` is lexed as a `GuardedStr { n_end_hashes: 3, .. }`
    // followed by a `#` token.
    let mut n_end_hashes = 0;
    while cursor.first() == '#' && n_end_hashes < n_start_hashes {
        n_end_hashes += 1;
        cursor.bump();
    }

    // Reserved syntax, always an error, so it doesn't matter if
    // `n_start_hashes != n_end_hashes`.

    eat_literal_suffix(cursor);

    let token_len = cursor.pos_within_token();
    cursor.reset_pos_within_token();

    Some(GuardedStr {
        n_hashes: n_start_hashes,
        terminated: true,
        token_len,
    })
}

/* #region RawString */

#[derive(
    Copy, Debug, PartialEq, Eq, PartialOrd, Ord, PanicFmt, derive_more::Display, thiserror::Error,
)]
#[derive_const(Clone)]
// cSpell:disable-next-line
#[pfmt(crate = konst::const_panic)]
pub enum RawStrError {
    /// Non `#` characters exist between `r` and `"`, e.g. `r##~"abcde"##`
    #[display("InvalidStarter {{ bad_char: {bad_char} }}")]
    InvalidStarter { bad_char: char },
    /// The string was not terminated, e.g. `r###"abcde"##`.
    /// `possible_terminator_offset` is the number of characters after `r` or
    /// `br` where they may have intended to terminate it.
    #[display(
        "Unterminated {{ expected: {expected}, found: {found}, possible_terminator_offset: {} }}",
        possible_terminator_offset.as_ref().map(|x| x.to_string()).unwrap_or("None".to_owned())
    )]
    NoTerminator {
        expected: u32,
        found: u32,
        possible_terminator_offset: Option<u32>,
    },

    /// More than 255 `#`s exist.
    #[display("TooManyDelimiters {{ found: {found} }}")]
    TooManyDelimiters { found: u32 },
}

/// Validates a raw string literal. Used for getting more information about a
/// problem with a `RawStr`/`RawByteStr` with a `None` field.
#[inline]
#[allow(unused)]
pub fn validate_raw_str(input: &str, prefix_len: u32) -> Result<(), RawStrError> {
    debug_assert!(!input.is_empty());
    let mut cursor = Cursor::new(input);
    // Move past the leading `r` or `br`.
    for _ in 0..prefix_len {
        cursor.bump().unwrap();
    }
    raw_double_quoted_string(&mut cursor, prefix_len).map(|_| ())
}

/// Eats the double-quoted string and returns `n_hashes` and an error if encountered.
fn raw_double_quoted_string(
    cursor: &mut Cursor<'_, TokenKind>,
    prefix_len: u32,
) -> Result<u8, RawStrError> {
    // Wrap the actual function to handle the error with too many hashes.
    // This way, it eats the whole raw string.
    let n_hashes = raw_string_unvalidated(cursor, prefix_len)?;
    // Only up to 255 `#`s are allowed in raw strings
    match u8::try_from(n_hashes) {
        Ok(num) => Ok(num),
        Err(_) => Err(RawStrError::TooManyDelimiters { found: n_hashes }),
    }
}

fn raw_string_unvalidated(
    cursor: &mut Cursor<'_, TokenKind>,
    prefix_len: u32,
) -> Result<u32, RawStrError> {
    debug_assert!(cursor.prev() == 'r');
    let start_pos = cursor.pos_within_token();
    let mut possible_terminator_offset = None;
    let mut max_hashes = 0;

    // Count opening '#' symbols.
    let mut eaten = 0;
    while cursor.first() == '#' {
        eaten += 1;

        // This fucking statement is used to avoid moving.
        cursor.bump();
    }

    let n_start_hashes = eaten;

    // Check that string is started.
    match cursor.bump() {
        Some('"') => (),
        c => {
            let c = c.unwrap_or(EOF_CHAR);
            return Err(RawStrError::InvalidStarter { bad_char: c });
        }
    }

    // Skip the string contents and on each '#' character met, check if this is
    // a raw string termination.
    loop {
        cursor.eat_until(b'"');

        if cursor.is_eof() {
            return Err(RawStrError::NoTerminator {
                expected: n_start_hashes,
                found: max_hashes,
                possible_terminator_offset,
            });
        }

        // Eat closing double quote.
        cursor.bump();

        // Check that amount of closing '#' symbols
        // is equal to the amount of opening ones.
        // Note that this will not consume extra trailing `#` characters:
        // `r###"abcde"####` is lexed as a `RawStr { n_hashes: 3 }`
        // followed by a `#` token.
        let mut n_end_hashes = 0;
        while cursor.first() == '#' && n_end_hashes < n_start_hashes {
            n_end_hashes += 1;
            cursor.bump();
        }

        if n_end_hashes == n_start_hashes {
            return Ok(n_start_hashes);
        } else if n_end_hashes > max_hashes {
            // Keep track of possible terminators to give a hint about
            // where there might be a missing terminator
            possible_terminator_offset =
                Some(cursor.pos_within_token() - start_pos - n_end_hashes + prefix_len);
            max_hashes = n_end_hashes;
        }
    }
}

/* #endregion */

fn byte_string(
    cursor: &mut Cursor<'_, TokenKind>,
    make_kind: fn(bool) -> LiteralKind,
    make_kind_raw: fn(Option<u8>) -> LiteralKind,
    single_quoted: Option<fn(bool) -> LiteralKind>,
) -> TokenKind {
    match (cursor.first(), cursor.second(), single_quoted) {
        ('\'', _, Some(single_quoted)) => {
            cursor.bump();
            let terminated = single_quoted_string(cursor);
            let suffix_start = cursor.pos_within_token();
            if terminated {
                eat_literal_suffix(cursor);
            }
            let kind = single_quoted(terminated);
            TokenKind::Literal { kind, suffix_start }
        }
        ('"', _, _) => {
            cursor.bump();
            let terminated = double_quoted_string(cursor);
            let suffix_start = cursor.pos_within_token();
            if terminated {
                eat_literal_suffix(cursor);
            }
            let kind = make_kind(terminated);
            TokenKind::Literal { kind, suffix_start }
        }
        ('r', '"', _) | ('r', '#', _) => {
            cursor.bump();
            let res = raw_double_quoted_string(cursor, 2);
            let suffix_start = cursor.pos_within_token();
            if res.is_ok() {
                eat_literal_suffix(cursor);
            }
            let kind = make_kind_raw(res.ok());
            TokenKind::Literal { kind, suffix_start }
        }
        _ => identifier_or_unknown_prefix(cursor),
    }
}

/* #endregion */

/* #region Number */

fn number(cursor: &mut Cursor<'_, TokenKind>, first_digit: char) -> LiteralKind {
    debug_assert!('0' <= cursor.prev() && cursor.prev() <= '9');
    let mut base = Base::Decimal;
    if first_digit == '0' {
        // Attempt to parse encoding base.
        match cursor.first() {
            'b' => {
                base = Base::Binary;
                cursor.bump();
                if !eat_decimal_digits(cursor) {
                    return LiteralKind::Int {
                        base,
                        empty_int: true,
                    };
                }
            }
            'o' => {
                base = Base::Octal;
                cursor.bump();
                if !eat_decimal_digits(cursor) {
                    return LiteralKind::Int {
                        base,
                        empty_int: true,
                    };
                }
            }
            'x' => {
                base = Base::Hexadecimal;
                cursor.bump();
                if !eat_hexadecimal_digits(cursor) {
                    return LiteralKind::Int {
                        base,
                        empty_int: true,
                    };
                }
            }
            // Not a base prefix; consume additional digits.
            '0'..='9' | '_' => {
                eat_decimal_digits(cursor);
            }

            // Also not a base prefix; nothing more to do here.
            '.' | 'e' | 'E' => {}

            // Just a 0.
            _ => {
                return LiteralKind::Int {
                    base,
                    empty_int: false,
                };
            }
        }
    } else {
        // No base prefix, parse number in the usual way.
        eat_decimal_digits(cursor);
    }

    match cursor.first() {
        // Don't be greedy if this is actually an
        // integer literal followed by field/method access or a range pattern
        // (`0..2` and `12.foo()`)
        '.' if cursor.second() != '.' && !is_rust_id_start(cursor.second()) => {
            // might have stuff after the ., and if it does, it needs to start
            // with a number
            cursor.bump();
            let mut empty_exponent = false;
            if cursor.first().is_ascii_digit() {
                eat_decimal_digits(cursor);
                match cursor.first() {
                    'e' | 'E' => {
                        cursor.bump();
                        empty_exponent = !eat_float_exponent(cursor);
                    }
                    _ => (),
                }
            }
            LiteralKind::Float {
                base,
                empty_exponent,
            }
        }
        'e' | 'E' => {
            cursor.bump();
            let empty_exponent = !eat_float_exponent(cursor);
            LiteralKind::Float {
                base,
                empty_exponent,
            }
        }
        _ => LiteralKind::Int {
            base,
            empty_int: false,
        },
    }
}

/* #endregion */

/* #region LiteralHelpers */

fn eat_decimal_digits(cursor: &mut Cursor<'_, TokenKind>) -> bool {
    let mut has_digits = false;
    loop {
        match cursor.first() {
            '_' => {
                cursor.bump();
            }
            '0'..='9' => {
                has_digits = true;
                cursor.bump();
            }
            _ => break,
        }
    }
    has_digits
}

fn eat_hexadecimal_digits(cursor: &mut Cursor<'_, TokenKind>) -> bool {
    let mut has_digits = false;
    loop {
        match cursor.first() {
            '_' => {
                cursor.bump();
            }
            '0'..='9' | 'a'..='f' | 'A'..='F' => {
                has_digits = true;
                cursor.bump();
            }
            _ => break,
        }
    }
    has_digits
}

/// Eats the float exponent. Returns true if at least one digit was met,
/// and returns false otherwise.
fn eat_float_exponent(cursor: &mut Cursor<'_, TokenKind>) -> bool {
    debug_assert!(cursor.prev() == 'e' || cursor.prev() == 'E');
    if cursor.first() == '-' || cursor.first() == '+' {
        cursor.bump();
    }
    eat_decimal_digits(cursor)
}

/// Eats the suffix of the literal, e.g. "u8".
fn eat_literal_suffix(cursor: &mut Cursor<'_, TokenKind>) {
    eat_identifier(cursor);
}

/* #endregion */
