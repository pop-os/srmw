use crate::MultiWriter;
use futures::{prelude::*, stream::FuturesUnordered};
use genawaiter::rc::Gen;
use std::{
    io::{self, SeekFrom},
    iter::FromIterator,
};

impl<W: AsyncSeek + Unpin> MultiWriter<W> {
    /// Seek all writers to a specific point.
    pub fn seek<'a>(&'a mut self, from: SeekFrom) -> impl Stream<Item = (usize, io::Error)> + 'a {
        Gen::new(|co| {
            async move {
                let &mut Self { ref mut subscribed, .. } = self;

                let mut stream = FuturesUnordered::from_iter(
                    subscribed
                        .iter_mut()
                        .map(|(indice, writer)| async move { (indice, writer.seek(from).await) }),
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
