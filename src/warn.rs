use std::error::Error;
use std::marker::PhantomData;

pub trait Sink<E: Error> {
    fn warn(&mut self, error: E);
}

pub struct Adapt<'a, E, S>(&'a mut S, PhantomData<E>);

pub struct AdaptMap<'a, D, E, F, S>(&'a mut S, F, PhantomData<D>, PhantomData<E>);

pub trait SinkExt<E: Error>: Sink<E> {
    #[inline]
    fn adapt(&mut self) -> Adapt<'_, E, Self>
    where
        Self: Sized,
    {
        Adapt(self, PhantomData)
    }

    #[inline]
    fn adapt_map<D, F>(&mut self, func: F) -> AdaptMap<'_, D, E, F, Self>
    where
        Self: Sized,
        D: Error,
        F: FnMut(D) -> E,
    {
        AdaptMap(self, func, PhantomData, PhantomData)
    }
}

impl<E: Error, S: Sink<E>> SinkExt<E> for S {}

impl<'a, E, F, S> Sink<E> for Adapt<'a, F, S>
where
    E: Error,
    F: Error + From<E>,
    S: Sink<F>,
{
    #[inline]
    fn warn(&mut self, error: E) {
        self.0.warn(F::from(error))
    }
}

impl<'a, D, E, F, S> Sink<D> for AdaptMap<'a, D, E, F, S>
where
    D: Error,
    E: Error,
    F: FnMut(D) -> E,
    S: Sink<E>,
{
    #[inline]
    fn warn(&mut self, error: D) {
        self.0.warn(self.1(error))
    }
}

#[derive(Debug)]
pub struct Ignore;

impl<E: Error> Sink<E> for Ignore {
    #[inline]
    fn warn(&mut self, _error: E) {}
}

#[derive(Debug)]
pub struct Stderr;

impl<E: Error> Sink<E> for Stderr {
    #[inline]
    fn warn(&mut self, error: E) {
        eprintln!("error: {}", error);
    }
}

#[derive(Debug)]
pub struct All<E: Error>(pub Vec<E>);

impl<E: Error> Default for All<E> {
    #[inline]
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<E: Error> Sink<E> for All<E> {
    #[inline]
    fn warn(&mut self, error: E) {
        self.0.push(error);
    }
}

#[derive(Debug)]
pub struct First<E: Error>(pub Option<E>);

impl<E: Error> Default for First<E> {
    #[inline]
    fn default() -> Self {
        Self(None)
    }
}

impl<E: Error> Sink<E> for First<E> {
    #[inline]
    fn warn(&mut self, error: E) {
        if self.0.is_none() {
            self.0 = Some(error);
        }
    }
}

#[derive(Debug)]
pub struct Last<E: Error>(pub Option<E>);

impl<E: Error> Default for Last<E> {
    #[inline]
    fn default() -> Self {
        Self(None)
    }
}

impl<E: Error> Sink<E> for Last<E> {
    #[inline]
    fn warn(&mut self, error: E) {
        self.0 = Some(error);
    }
}

pub struct FromFn<E: Error, F: FnMut(E)>(F, PhantomData<E>);

#[inline]
pub fn from_fn<E: Error, F: FnMut(E)>(func: F) -> FromFn<E, F> {
    FromFn(func, PhantomData)
}

impl<E: Error, F: FnMut(E)> Sink<E> for FromFn<E, F> {
    #[inline]
    fn warn(&mut self, error: E) {
        self.0(error)
    }
}

pub trait OptionExt {
    fn or_warn_with<E: Error>(self, error: E, warn: &mut impl Sink<E>) -> Self;
}

pub trait ResultExt<E: Error> {
    fn or_warn<F: From<E> + Error>(self, warn: &mut impl Sink<F>) -> Self;
    fn or_warn_map<F: Error>(self, func: impl FnOnce(E) -> F, warn: &mut impl Sink<F>) -> Self;
}

impl<T> OptionExt for Option<T> {
    #[inline]
    fn or_warn_with<E: Error>(self, error: E, warn: &mut impl Sink<E>) -> Self {
        if self.is_none() {
            warn.warn(error);
        }
        self
    }
}

impl<T, E: Error + Clone> ResultExt<E> for Result<T, E> {
    #[inline]
    fn or_warn<F: From<E> + Error>(self, warn: &mut impl Sink<F>) -> Self {
        self.or_warn_map(From::from, warn)
    }

    #[inline]
    fn or_warn_map<F: Error>(self, func: impl FnOnce(E) -> F, warn: &mut impl Sink<F>) -> Self {
        if let Err(e) = &self {
            warn.warn(func(e.clone()));
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use thiserror::Error;

    #[derive(Debug, Error, Eq, PartialEq)]
    #[error("first: {value}")]
    struct ErrFirst {
        value: usize,
    }

    #[derive(Debug, Error, Eq, PartialEq)]
    #[error("second: {0}")]
    struct ErrSecond(#[from] ErrFirst);

    fn recursive(n: usize, warn: &mut impl Sink<ErrFirst>) {
        if n == 0 {
            return;
        }
        recursive(n - 1, warn);
        warn.warn(ErrFirst { value: n });
        recursive(n - 1, warn);
    }

    #[test]
    fn test_recursive() {
        let mut sink = All::default();
        recursive(3, &mut sink);
        let res = vec![
            ErrFirst { value: 1 },
            ErrFirst { value: 2 },
            ErrFirst { value: 1 },
            ErrFirst { value: 3 },
            ErrFirst { value: 1 },
            ErrFirst { value: 2 },
            ErrFirst { value: 1 },
        ];
        assert_eq!(sink.0, res);
    }

    fn inner(warn: &mut impl Sink<ErrFirst>) {
        warn.warn(ErrFirst { value: 1 });
    }

    fn outer(warn: &mut impl Sink<ErrSecond>) {
        inner(&mut warn.adapt());
        warn.warn(ErrSecond(ErrFirst { value: 2 }));
    }

    #[test]
    fn test_adapt() {
        let mut sink = All::default();
        outer(&mut sink);
        let res = vec![
            ErrSecond(ErrFirst { value: 1 }),
            ErrSecond(ErrFirst { value: 2 }),
        ];
        assert_eq!(sink.0, res);
    }
}
