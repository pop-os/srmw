use async_std::fs::{File, OpenOptions};
use futures::{executor, prelude::*, stream::FuturesOrdered};
use srmw::*;

use std::{
    io::{self, SeekFrom},
    iter::FromIterator,
    time::Instant,
};

fn main() {
    executor::block_on(async move {
        let mut original = File::open("examples/original").await.unwrap();

        let mut writers = MultiWriter::default();

        let paths = [
            "examples/copy1",
            "examples/copy2",
            "examples/copy3",
            "examples/copy4",
            "examples/copy5",
            "examples/copy6",
            "examples/copy7",
            "examples/copy8",
        ];

        let mut newly_created = FuturesOrdered::from_iter(
            paths.iter().map(|path| async move { create(path).await.unwrap() }),
        );

        while let Some(file) = newly_created.next().await {
            writers.insert(file);
        }

        let buf = &mut [0u8; 64 * 1024];

        println!("creating copies");

        let mut start = Instant::now();

        {
            let mut generator = writers.copy(&mut original, buf);

            while let Some(event) = generator.next().await {
                match event {
                    CopyEvent::Progress(_read) => (),

                    CopyEvent::Failure(indice, why) => {
                        eprintln!("copy error: file {}: {}", paths[indice], why);
                    }

                    CopyEvent::NoWriters => {
                        eprintln!("no writers left to copy to");
                        return;
                    }

                    CopyEvent::SourceFailure(why) => {
                        eprintln!("failed to read from source: {}", why);
                        return;
                    }
                }
            }
        }

        println!("flushing copies to disk");

        {
            let mut generator = writers.flush();

            while let Some((indice, error)) = generator.next().await {
                eprintln!("seek error: file {}: {:?}", indice, error);
            }
        }

        println!("copied in {:?}", Instant::now().duration_since(start));

        println!("seeking files to the beginning again");

        {
            let mut generator = writers.seek(SeekFrom::Start(0));

            while let Some((indice, why)) = generator.next().await {
                eprintln!("seek error: file {}: {}", paths[indice], why);
            }

            let _ = original.seek(SeekFrom::Start(0)).await;
        }

        println!("validating the the copied filed");

        start = Instant::now();

        {
            let copy_bufs = &mut Vec::new();
            let mut generator = writers.validate(&mut original, buf, copy_bufs);

            while let Some(event) = generator.next().await {
                match event {
                    ValidationEvent::Failure(indice, why) => {
                        eprintln!("validation error: file {}: {}", paths[indice], why);
                    }
                    ValidationEvent::NoWriters => {
                        eprintln!("no writers left to validate");
                        return;
                    }
                    ValidationEvent::Progress(_read) => (),
                    ValidationEvent::SourceFailure(why) => {
                        eprintln!("failed to read from source: {}", why);
                        return;
                    }
                }
            }
        }

        println!("validated in {:?}", Instant::now().duration_since(start));
        println!("The following files were successfully written and verified:");

        for (indice, _) in writers.into_iter() {
            println!("    {}", paths[indice]);
        }
    });
}

async fn create(path: &str) -> io::Result<File> {
    OpenOptions::new().create(true).read(true).write(true).open(path).await
}
