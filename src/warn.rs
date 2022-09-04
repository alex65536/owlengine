use std::error::Error;
use std::marker::PhantomData;

pub trait Sink<E: Error> {
    fn warn(&mut self, error: E);
}

pub struct Adapt<'a, E: Error, S: Sink<E>>(&'a mut S, PhantomData<E>);

pub fn adapt<'a, E: Error, S: Sink<E>>(sink: &'a mut S) -> Adapt<'a, E, S> {
    Adapt(sink, PhantomData)
}

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

    fn recursive(n: usize, sink: &mut impl Sink<ErrFirst>) {
        if n == 0 {
            return;
        }
        recursive(n-1, sink);
        sink.warn(ErrFirst {value: n});
        recursive(n-1, sink);
    }

    #[test]
    fn test_recursive() {
        let mut sink = All::default();
        recursive(3, &mut sink);
        let res = vec![
            ErrFirst {value: 1},
            ErrFirst {value: 2},
            ErrFirst {value: 1},
            ErrFirst {value: 3},
            ErrFirst {value: 1},
            ErrFirst {value: 2},
            ErrFirst {value: 1},
        ];
        assert_eq!(sink.0, res);
    }

    fn inner(sink: &mut impl Sink<ErrFirst>) {
        sink.warn(ErrFirst {value: 1});
    }

    fn outer(sink: &mut impl Sink<ErrSecond>) {
        inner(&mut adapt(sink));
        sink.warn(ErrSecond(ErrFirst {value: 2}));
    }

    #[test]
    fn test_adapt() {
        let mut sink = All::default();
        outer(&mut sink);
        let res = vec![
            ErrSecond(ErrFirst {value: 1}),
            ErrSecond(ErrFirst {value: 2}),
        ];
        assert_eq!(sink.0, res);
    }
}
