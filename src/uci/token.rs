use std::{borrow::Borrow, fmt::Display, num, ops::Deref};

use owlchess::{moves::UciMove, Board, Move, RawBoard};

use thiserror::Error;

#[derive(Debug, Clone, Error, Eq, PartialEq)]
pub enum Error {
    #[error("token is empty")]
    Empty,
    #[error("token contains whitespace")]
    Whitespace,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct Token(str);

impl Token {
    #[inline]
    pub unsafe fn new_unchecked(s: &str) -> &Token {
        &*(s as *const str as *const Token)
    }

    #[inline]
    pub fn new(s: &str) -> Result<&Token, Error> {
        if s.is_empty() {
            return Err(Error::Empty);
        }
        if s.chars().any(|c| c.is_whitespace()) {
            return Err(Error::Whitespace);
        }
        Ok(unsafe { Self::new_unchecked(s) })
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for Token {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for Token {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for &Token {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<Token> for str {
    #[inline]
    fn eq(&self, other: &Token) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<&Token> for str {
    #[inline]
    fn eq(&self, other: &&Token) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<str> for Token {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Token {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[inline]
pub fn tokenize(s: &str) -> impl Iterator<Item = &Token> {
    s.split_whitespace()
        .map(|tok| unsafe { Token::new_unchecked(tok) })
}

macro_rules! token_safe {
    ($($ty:ty),* $(,)?) => {
        $(
            unsafe impl TokenSafe for $ty {}
        )*
    };
}

pub unsafe trait TokenSafe: Display {}
pub unsafe trait MultiTokenSafe: Display {}

token_safe! {
    bool,
    i8, i16, i32, i64, i128,
    u8, u16, u32, u64, u128,
    num::NonZeroI8, num::NonZeroI16, num::NonZeroI32, num::NonZeroI64, num::NonZeroI128,
    num::NonZeroU8, num::NonZeroU16, num::NonZeroU32, num::NonZeroU64, num::NonZeroU128,
    Move, UciMove,
}

unsafe impl<T: TokenSafe> MultiTokenSafe for T {}
unsafe impl MultiTokenSafe for RawBoard {}
unsafe impl MultiTokenSafe for Board {}

pub trait PushTokens {
    fn push(&mut self, token: &Token);
    fn push_fmt<T: TokenSafe>(&mut self, value: &T);
    fn push_many_fmt<T: MultiTokenSafe>(&mut self, value: &T);

    #[inline]
    fn push_many(&mut self, tokens: &[&Token]) {
        for token in tokens {
            self.push(token);
        }
    }
}
