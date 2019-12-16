use crate::MultiWriter;
use futures::{prelude::*, stream::FuturesUnordered, AsyncRead, AsyncWrite};
use genawaiter::rc::Gen;
use std::{io, iter::FromIterator};

#[derive(Debug)]
pub enum CopyEvent {
    /// One of the outputs failed to validate.
    Failure(usize, io::Error),
    /// Either no writers were given, or all writers have failed.
    NoWriters,
    /// This many bytes have been read from the source.
    Progress(usize),
    /// The entire process has failed due to a source read error.
    SourceFailure(io::Error),
}

impl<W: AsyncWrite + Unpin> MultiWriter<W> {
    /// Copies bytes from the source to each writer concurrently.
    ///
    /// # Notes
    ///
    /// When a writer fails, it is removed from the slab.
    pub fn copy<'a, R: AsyncRead + Unpin + 'a>(
        &'a mut self,
        mut reader: R,
        buf: &'a mut [u8],
    ) -> impl Stream<Item = CopyEvent> + 'a {
        Gen::new(|co| {
            async move {
                let &mut Self { ref mut subscribed, ref mut drop_list } = self;
                let mut read;

                loop {
                    for indice in drop_list.drain(..) {
                        subscribed.remove(indice);
                    }

                    if subscribed.is_empty() {
                        co.yield_(CopyEvent::NoWriters).await;
                        break;
                    }

                    read = match reader.read(buf).await {
                        Ok(read) => read,
                        Err(why) => {
                            co.yield_(CopyEvent::SourceFailure(why)).await;
                            break;
                        }
                    };

                    if read == 0 {
                        break;
                    }

                    let slice = &buf[..read];

                    let mut stream = FuturesUnordered::from_iter(subscribed.iter_mut().map(
                        |(indice, writer)| async move { (indice, writer.write_all(slice).await) },
                    ));

                    while let Some((indice, why)) = stream.next().await {
                        if let Err(why) = why {
                            co.yield_(CopyEvent::Failure(indice, why)).await;
                            drop_list.push(indice);
                        }
                    }

                    co.yield_(CopyEvent::Progress(read)).await;
                }
            }
        })
    }
}
