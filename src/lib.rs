#![feature(async_iterator, async_closure, gen_blocks, async_for_loop, extend_one)]
#![allow(async_fn_in_trait)]

use std::{async_iter::AsyncIterator, pin::Pin};

pub trait AsyncIteratorExt: AsyncIterator {
    fn map<T, F>(self, mut f: F) -> impl AsyncIterator<Item = T>
    where
        F: FnMut(Self::Item) -> T,
        Self: Sized,
    {
        async gen move {
            for await x in self {
                yield f(x);
            }
        }
    }

    fn enumerate(self) -> impl AsyncIterator<Item = (usize, Self::Item)>
    where
        Self: Sized,
    {
        async gen move {
            let mut idx = 0;
            for await x in self {
                yield (idx, x);
                idx += 1;
            }
        }
    }

    fn filter<F>(self, mut f: F) -> impl AsyncIterator<Item = Self::Item>
    where
        F: async FnMut(&Self::Item) -> bool,
        Self: Sized,
    {
        async gen move {
            for await x in self {
                if f(&x).await {
                    yield x;
                }
            }
        }
    }

    fn filter_map<T, F>(self, mut f: F) -> impl AsyncIterator<Item = T>
    where
        F: async FnMut(Self::Item) -> Option<T>,
        Self: Sized,
    {
        async gen move {
            for await x in self {
                if let Some(y) = f(x).await {
                    yield y;
                }
            }
        }
    }

    fn then<F, T>(self, mut f: F) -> impl AsyncIterator<Item = T>
    where
        F: async FnMut(Self::Item) -> T,
        Self: Sized,
    {
        async gen move {
            for await x in self {
                yield f(x).await;
            }
        }
    }

    async fn collect<C>(self) -> C
    where
        C: Default + Extend<Self::Item>,
        Self: Sized,
    {
        let mut xs = C::default();
        for await x in self {
            xs.extend_one(x); // <- no stable extend_one :(
        }
        xs
    }

    async fn unzip<A, B, FromA, FromB>(self) -> (FromA, FromB)
    where
        FromA: Default + Extend<A>,
        FromB: Default + Extend<B>,
        Self: Sized + AsyncIterator<Item = (A, B)>,
    {
        let mut xas = FromA::default();
        let mut xbs = FromB::default();
        for await (xa, xb) in self {
            xas.extend_one(xa);
            xbs.extend_one(xb);
        }
        (xas, xbs)
    }

    async fn concat(self) -> Self::Item
    where
        Self: Sized,
        Self::Item: Extend<<Self::Item as IntoIterator>::Item> + IntoIterator + Default,
    {
        let mut cat = Self::Item::default();
        for await x in self {
            cat.extend(x);
        }
        cat
    }

    async fn count(self) -> usize
    where
        Self: Sized,
    {
        let mut count = 0;
        for await _ in self {
            count += 1;
        }
        count
    }

    fn cycle(self) -> impl AsyncIterator<Item = Self::Item>
    where
        Self: Sized + Clone,
    {
        async gen move {
            loop {
                for await x in self.clone() {
                    yield x;
                }
            }
        }
    }

    async fn fold<T, F>(self, mut init: T, mut f: F) -> T
    where
        F: async FnMut(T, Self::Item) -> T,
        Self: Sized,
    {
        for await x in self {
            init = f(init, x).await;
        }
        init
    }

    async fn any<F>(self, mut f: F) -> bool
    where
        F: async FnMut(Self::Item) -> bool,
        Self: Sized,
    {
        for await x in self {
            if f(x).await {
                return true;
            }
        }
        false
    }

    async fn all<F>(self, mut f: F) -> bool
    where
        F: async FnMut(Self::Item) -> bool,
        Self: Sized,
    {
        for await x in self {
            if !f(x).await {
                return false;
            }
        }
        true
    }

    fn flatten(self) -> impl AsyncIterator<Item = <Self::Item as AsyncIterator>::Item>
    where
        Self::Item: AsyncIterator,
        Self: Sized,
    {
        async gen move {
            for await x in self {
                for await y in x {
                    yield y;
                }
            }
        }
    }

    fn scan<S, B, F>(self, mut initial_state: S, mut f: F) -> impl AsyncIterator<Item = B>
    where
        F: async FnMut(&mut S, Self::Item) -> Option<B>,
        Self: Sized,
    {
        async gen move {
            for await x in self {
                if let Some(b) = f(&mut initial_state, x).await {
                    yield b;
                } else {
                    break;
                }
            }
        }
    }

    fn skip_while<F>(self, mut f: F) -> impl AsyncIterator<Item = Self::Item>
    where
        F: async FnMut(&Self::Item) -> bool,
        Self: Sized,
    {
        async gen move {
            let mut skip = true;
            for await x in self {
                if skip {
                    if !f(&x).await {
                        yield x;
                        skip = false;
                    }
                } else {
                    yield x;
                }
            }
        }
    }

    fn take_while<F>(self, mut f: F) -> impl AsyncIterator<Item = Self::Item>
    where
        F: async FnMut(&Self::Item) -> bool,
        Self: Sized,
    {
        async gen move {
            for await x in self {
                if f(&x).await {
                    yield x;
                } else {
                    break;
                }
            }
        }
    }

    async fn for_each<F>(self, mut f: F)
    where
        F: async FnMut(Self::Item),
        Self: Sized,
    {
        for await x in self {
            f(x).await;
        }
    }

    fn take(self, mut n: usize) -> impl AsyncIterator<Item = Self::Item>
    where
        Self: Sized,
    {
        async gen move {
            if n == 0 {
                return;
            }

            for await x in self {
                yield x;

                n -= 1;
                if n == 0 {
                    break;
                }
            }
        }
    }

    fn skip(self, mut n: usize) -> impl AsyncIterator<Item = Self::Item>
    where
        Self: Sized,
    {
        async gen move {
            for await x in self {
                if n == 0 {
                    yield x;
                } else {
                    n -= 1;
                }
            }
        }
    }

    fn boxed<'a>(self) -> Pin<Box<dyn AsyncIterator<Item = Self::Item> + Send + 'a>>
    where
        Self: Sized + Send + 'a,
    {
        Box::pin(self)
    }

    fn boxed_local<'a>(self) -> Pin<Box<dyn AsyncIterator<Item = Self::Item> + 'a>>
    where
        Self: Sized + 'a,
    {
        Box::pin(self)
    }

    fn chain<St>(self, other: St) -> impl AsyncIterator<Item = Self::Item>
    where
        St: AsyncIterator<Item = Self::Item>,
        Self: Sized,
    {
        async gen move {
            for await x in self {
                yield x;
            }
            for await x in other {
                yield x;
            }
        }
    }
}

impl<T: AsyncIterator> AsyncIteratorExt for T {}
