use crate::MultiWriter;
use futures::{prelude::*, stream::FuturesUnordered, AsyncRead};
use genawaiter::rc::Gen;
use std::{io, iter::FromIterator};

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("error while reading from copy")]
    Read(#[source] io::Error),
    #[error("mismatch between copy and source")]
    Mismatch,
}

#[derive(Debug)]
pub enum ValidationEvent {
    /// One of the outputs failed to validate.
    Failure(usize, ValidationError),
    /// Either no writers were given, or all writers have failed.
    NoWriters,
    /// This many bytes have been read from the source.
    Progress(usize),
    /// The entire process has failed due to a source read error.
    SourceFailure(io::Error),
}

impl<W: AsyncRead + Unpin> MultiWriter<W> {
    /// Validates that the contents of the source match that of the writers.
    pub fn validate<'a, R: AsyncRead + Unpin + 'a>(
        &'a mut self,
        mut source: R,
        source_buf: &'a mut [u8],
        copy_bufs: &'a mut Vec<Vec<u8>>,
    ) -> impl Stream<Item = ValidationEvent> + 'a {
        Gen::new(|co| {
            async move {
                let &mut Self { ref mut subscribed, ref mut drop_list } = self;

                // Ensure that the buffer for each copy is the same size as the source buffer.
                for buffer in copy_bufs.iter_mut() {
                    if buffer.capacity() < source_buf.len() {
                        buffer.reserve_exact(source_buf.len() - buffer.capacity());
                    }
                }

                // Ensure that there are as many copy buffers as copies.
                if copy_bufs.len() < subscribed.len() {
                    for _ in 0..subscribed.len() - copy_bufs.len() {
                        copy_bufs.push(vec![0; source_buf.len()]);
                    }
                }

                let mut read;
                loop {
                    for indice in drop_list.drain(..) {
                        subscribed.remove(indice);
                    }

                    if subscribed.is_empty() {
                        co.yield_(ValidationEvent::NoWriters).await;
                        break;
                    }

                    read = match source.read(source_buf).await {
                        Ok(read) => read,
                        Err(why) => {
                            co.yield_(ValidationEvent::SourceFailure(why)).await;
                            break;
                        }
                    };

                    if read == 0 {
                        break;
                    }

                    let slice = &source_buf[..read];

                    let mut stream = FuturesUnordered::from_iter(
                        subscribed.iter_mut().zip(copy_bufs.iter_mut()).map(
                            |((indice, copy), buf)| {
                                async move {
                                    let this_slice = &mut buf[..read];

                                    if let Err(why) = copy.read_exact(this_slice).await {
                                        return Err((indice, ValidationError::Read(why)));
                                    }

                                    if this_slice == slice {
                                        Ok(())
                                    } else {
                                        Err((indice, ValidationError::Mismatch))
                                    }
                                }
                            },
                        ),
                    );

                    while let Some(result) = stream.next().await {
                        if let Err((indice, why)) = result {
                            co.yield_(ValidationEvent::Failure(indice, why)).await;
                            drop_list.push(indice);
                        }
                    }

                    co.yield_(ValidationEvent::Progress(read)).await;
                }
            }
        })
    }
}
