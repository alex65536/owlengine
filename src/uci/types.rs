use thiserror::Error;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Permille(u16);

#[derive(Debug, Clone, Error, Eq, PartialEq)]
pub enum PermilleError {
    #[error("value is less than 0")]
    TooSmall,
    #[error("value is greater than 1")]
    TooLarge,
}

impl Permille {
    #[inline]
    pub fn new(amount: u16) -> Self {
        assert!(amount <= 1000, "permille amount must be between 0 and 1000");
        Self(amount)
    }

    #[inline]
    pub fn new_truncated(amount: u64) -> Self {
        Self(amount.min(1000) as u16)
    }

    #[inline]
    pub fn amount(&self) -> u16 {
        self.0
    }
}

impl From<Permille> for f32 {
    #[inline]
    fn from(p: Permille) -> Self {
        p.0 as Self / 1000.0
    }
}

impl From<Permille> for f64 {
    #[inline]
    fn from(p: Permille) -> Self {
        p.0 as Self / 1000.0
    }
}

impl TryFrom<f64> for Permille {
    type Error = PermilleError;

    fn try_from(v: f64) -> Result<Self, Self::Error> {
        match v {
            _ if v < 0.0 => Err(PermilleError::TooSmall),
            _ if v > 1.0 => Err(PermilleError::TooLarge),
            _ => Ok(Permille((v * 1000.0).round() as u16)),
        }
    }
}

impl TryFrom<f32> for Permille {
    type Error = PermilleError;

    fn try_from(v: f32) -> Result<Self, Self::Error> {
        TryFrom::try_from(v as f64)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TriStatus {
    Ok,
    Checking,
    Error,
}
