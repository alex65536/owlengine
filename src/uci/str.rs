use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
    str::FromStr,
};

use thiserror::Error;

macro_rules! impl_uci_str {
    ($name:ident, $bad_tokens:expr) => {
        impl FromStr for $name {
            type Err = Error;

            #[inline]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                from_str_impl(s, &[]).map(Self)
            }
        }

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
                self.0.as_str()
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

#[derive(Debug, Clone, Error, Eq, PartialEq)]
pub enum Error {
    #[error("string contains bad token \"{0}\"")]
    BadToken(&'static str),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct UciStr(String);

impl_uci_str! {UciStr, &[]}

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
