use std::{
    borrow::Borrow,
    cmp::Ordering,
    convert::Infallible,
    fmt::{self, Write},
    hash::{Hash, Hasher},
    ops::Deref,
    str::FromStr,
};

use super::token::{MultiTokenSafe, PushTokens, Token, TokenSafe};

use thiserror::Error;

macro_rules! impl_uci_str_base {
    ($name:ident) => {
        impl fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        unsafe impl MultiTokenSafe for $name {}

        impl From<$name> for String {
            #[inline]
            fn from(val: $name) -> Self {
                val.0
            }
        }

        impl AsRef<str> for $name {
            #[inline]
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl AsRef<UciStr> for $name {
            #[inline]
            fn as_ref(&self) -> &UciStr {
                self.as_uci_str()
            }
        }

        impl $name {
            #[inline]
            pub fn new() -> Self {
                Self::default()
            }

            #[inline]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            #[inline]
            pub fn as_uci_str(&self) -> &UciStr {
                unsafe { &*(self.as_str() as *const str as *const UciStr) }
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
            pub fn from_tokens(tokens: &[&Token]) -> Result<Self, Error> {
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
        impl Deref for $name {
            type Target = str;

            #[inline]
            fn deref(&self) -> &Self::Target {
                self.as_str()
            }
        }

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

macro_rules! impl_case_sensitive {
    ($name:ident) => {
        impl Deref for $name {
            type Target = UciStr;

            #[inline]
            fn deref(&self) -> &Self::Target {
                self.as_uci_str()
            }
        }

        impl Borrow<UciStr> for $name {
            #[inline]
            fn borrow(&self) -> &UciStr {
                self.as_uci_str()
            }
        }
    };
}

#[derive(Debug, Clone, Error, Eq, PartialEq)]
pub enum Error {
    #[error("string contains bad token \"{0}\"")]
    BadToken(&'static str),
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct UciStr(str);

impl UciStr {
    #[inline]
    fn to_uci_string(&self) -> UciString {
        UciString(self.0.to_owned())
    }
}

impl Deref for UciStr {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for UciStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

unsafe impl MultiTokenSafe for UciStr {}

impl UciStr {
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ToOwned for UciStr {
    type Owned = UciString;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        self.to_uci_string()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct UciString(String);

impl_uci_str_base! {UciString}
impl_case_sensitive! {UciString}

impl FromStr for UciString {
    type Err = Infallible;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(from_str_impl(s, &[]).unwrap()))
    }
}

impl From<&str> for UciString {
    #[inline]
    fn from(s: &str) -> Self {
        Self(from_str_impl(s, &[]).unwrap())
    }
}

impl UciString {
    #[inline]
    pub fn from_tokens(tokens: &[&Token]) -> Self {
        Self(tokens.join(" "))
    }

    #[inline]
    fn maybe_push_space(&mut self) {
        if !self.0.is_empty() {
            self.0 += " ";
        }
    }
}

impl PushTokens for UciString {
    #[inline]
    fn push(&mut self, token: &Token) {
        self.maybe_push_space();
        self.0 += token.as_str();
    }

    #[inline]
    fn push_fmt<T: TokenSafe>(&mut self, value: &T) {
        self.maybe_push_space();
        write!(self.0, "{}", value).expect("formatting failed");
    }

    #[inline]
    fn push_many_fmt<T: MultiTokenSafe>(&mut self, value: &T) {
        let orig_len = self.0.len();
        self.maybe_push_space();
        let spaced_len = self.0.len();
        write!(self.0, "{}", value).expect("formatting failed");
        if self.0.len() == spaced_len {
            self.0.truncate(orig_len);
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct RegisterName(String);

impl_uci_str! {RegisterName, &["code"]}
impl_case_sensitive! {RegisterName}

#[derive(Debug, Clone, Default)]
pub struct OptName(String);

impl_uci_str! {OptName, &["type", "value"]}
impl_case_insensitive! {OptName}

#[derive(Debug, Clone, Default)]
pub struct OptComboVar(String);

impl_uci_str! {OptComboVar, &["var"]}
impl_case_insensitive! {OptComboVar}

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
