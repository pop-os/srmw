use crate::MultiWriter;
use futures::{prelude::*, stream::FuturesUnordered, AsyncWrite};
use genawaiter::rc::Gen;

use std::{io, iter::FromIterator};

impl<W: AsyncWrite + Unpin> MultiWriter<W> {
    /// Flush all of the writers in the queue
    pub fn flush<'a>(&'a mut self) -> impl Stream<Item = (usize, io::Error)> + 'a {
        Gen::new(|co| {
            async move {
                let &mut Self { ref mut subscribed, .. } = self;

                let mut stream = FuturesUnordered::from_iter(
                    subscribed
                        .iter_mut()
                        .map(|(indice, writer)| async move { (indice, writer.flush().await) }),
                );

                while let Some((indice, why)) = stream.next().await {
                    if let Err(why) = why {
                        co.yield_((indice, why)).await;
                    }
                }
            }
        })
    }
}
