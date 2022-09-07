use std::{borrow::Borrow, ops::Deref};

use super::str::{UciStr, UciString};

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

pub trait PushTokens {
    fn push_token(&mut self, token: &Token);
    fn push_str(&mut self, str: &UciStr);

    #[inline]
    fn push_tokens(&mut self, tokens: &[&Token]) {
        for token in tokens {
            self.push_token(token);
        }
    }
}

impl PushTokens for UciString {
    #[inline]
    fn push_str(&mut self, str: &UciStr) {
        if str.is_empty() {
            return;
        }
        if !self.0.is_empty() {
            self.0 += " ";
        }
        self.0 += &str;
    }

    #[inline]
    fn push_token(&mut self, token: &Token) {
        if !self.0.is_empty() {
            self.0 += " ";
        }
        self.0 += token.as_str();
    }
}
