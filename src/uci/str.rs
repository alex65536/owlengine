use std::{
    borrow::Borrow,
    cmp::Ordering,
    convert::Infallible,
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
    str::FromStr,
};

use thiserror::Error;

macro_rules! impl_uci_str_base {
    ($name:ident) => {
        impl fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl Deref for $name {
            type Target = str;

            #[inline]
            fn deref(&self) -> &Self::Target {
                self.as_str()
            }
        }

        impl AsRef<str> for $name {
            #[inline]
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl $name {
            #[inline]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }
    };
}

macro_rules! impl_uci_str {
    ($name:ident, $bad_tokens:expr) => {
        impl_uci_str_base! {$name}

        impl FromStr for $name {
            type Err = Error;

            #[inline]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                from_str_impl(s, $bad_tokens).map(Self)
            }
        }

        impl $name {
            #[inline]
            pub fn from_tokens(tokens: &[&UciToken]) -> Result<Self, Error> {
                for token in tokens {
                    if let Some(&bad_token) = $bad_tokens.iter().find(|&t| t == &token) {
                        return Err(Error::BadToken(bad_token));
                    }
                }
                Ok(Self(tokens.join(" ")))
            }
        }
    };
}

macro_rules! impl_case_insensitive {
    ($name:ident) => {
        impl $name {
            #[inline]
            fn iter_low(&self) -> impl Iterator<Item = char> + '_ {
                self.0.chars().map(|c| c.to_ascii_lowercase())
            }
        }

        impl PartialEq for $name {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.iter_low().eq(other.iter_low())
            }
        }

        impl Eq for $name {}

        impl Ord for $name {
            #[inline]
            fn cmp(&self, other: &Self) -> Ordering {
                self.iter_low().cmp(other.iter_low())
            }
        }

        impl PartialOrd for $name {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Hash for $name {
            #[inline]
            fn hash<H: Hasher>(&self, state: &mut H) {
                for b in self.0.bytes() {
                    state.write_u8(b.to_ascii_lowercase());
                }
            }
        }
    };
}

pub trait PushTokens {
    fn push_token(&mut self, token: &UciToken);
    fn push_str(&mut self, str: &UciString);

    #[inline]
    fn push_tokens(&mut self, tokens: &[&UciToken]) {
        for token in tokens {
            self.push_token(token);
        }
    }
}

#[derive(Debug, Clone, Error, Eq, PartialEq)]
pub enum Error {
    #[error("string contains bad token \"{0}\"")]
    BadToken(&'static str),
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct UciToken(str);

impl UciToken {
    #[inline]
    pub unsafe fn from_str_unchecked(s: &str) -> &UciToken {
        &*(s as *const str as *const UciToken)
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for UciToken {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for UciToken {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for &UciToken {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<UciToken> for str {
    #[inline]
    fn eq(&self, other: &UciToken) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<&UciToken> for str {
    #[inline]
    fn eq(&self, other: &&UciToken) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<str> for UciToken {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for UciToken {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct UciString(String);

impl_uci_str_base! {UciString}

impl FromStr for UciString {
    type Err = Infallible;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(from_str_impl(s, &[]).unwrap()))
    }
}

impl UciString {
    #[inline]
    pub fn from_tokens(tokens: &[&UciToken]) -> Self {
        Self(tokens.join(" "))
    }
}

impl PushTokens for UciString {
    #[inline]
    fn push_str(&mut self, str: &UciString) {
        if str.is_empty() {
            return;
        }
        if !self.0.is_empty() {
            self.0 += " ";
        }
        self.0 += &str.0;
    }

    #[inline]
    fn push_token(&mut self, token: &UciToken) {
        if !self.0.is_empty() {
            self.0 += " ";
        }
        self.0 += token.as_str();
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RegisterName(String);

impl_uci_str! {RegisterName, &["code"]}

#[derive(Debug, Clone)]
pub struct OptName(String);

impl_uci_str! {OptName, &["type", "value"]}
impl_case_insensitive! {OptName}

#[derive(Debug, Clone)]
pub struct OptEnumValue(String);

impl_uci_str! {OptEnumValue, &["var"]}
impl_case_insensitive! {OptEnumValue}

#[inline]
fn from_str_impl(value: &str, bad_tokens: &[&'static str]) -> Result<String, Error> {
    let mut s = String::with_capacity(value.len());
    let mut first = true;
    for token in value.split_whitespace() {
        if !first {
            s += " ";
        }
        first = false;
        if let Some(&bad_token) = bad_tokens.iter().find(|&t| t == &token) {
            return Err(Error::BadToken(bad_token));
        }
        s += token;
    }
    Ok(s)
}

#[inline]
pub fn tokenize(s: &str) -> impl Iterator<Item = &UciToken> {
    s.split_whitespace()
        .map(|tok| unsafe { UciToken::from_str_unchecked(tok) })
}
