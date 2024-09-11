//! Helps to sort media.

use core::mem;

use crate::models::media::Media;
use crate::models::metadata::SpecificMetadata;

pub struct PreparedQuery {
    pub initial_select: String, // something like "SELECT * FROM info"
    pub where_clauses: Vec<(String,)>,
}

pub type WhereClause = String;
pub type Param = String;

/// Different sorts users can apply to a search.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SortType {
    /// No order at all. All elements are randomly sorted.
    Random,
    DateFirstSeen,
    DateModified,
    DateCreated,
    TagCount,
    Type,
    Size,
    Resolution,
    /// How long a video is. This will put all photos at the end.
    Duration,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SortOrder {
    /// Lowest value first.
    ///
    /// filesize ex: `[2_B, 1_GiB, 44_MiB].sort(SortOrder::Ascending)``
    /// `=> [2_B, 44_MiB, 1_GiB]`
    Ascending,
    /// Highest value first.
    ///
    /// filesize ex: `[2_B, 1_GiB, 44_MiB].sort(SortOrder::Descending)``
    /// `=> [1_GiB, 44_MiB, 2_B]`
    Descending,
}

/// A query that has been executed and can now be sorted based on user input.
pub struct FinishedQuery(Vec<Media>);

use rand::seq::SliceRandom;
use rand::thread_rng;

impl FinishedQuery {
    pub async fn sort(&mut self, ty: SortType, order: SortOrder) {
        let v = &mut self.0;

        match ty {
            SortType::Random => v.shuffle(&mut thread_rng()),
            SortType::DateFirstSeen => {
                v.sort_by(|a, b| a.metadata.first_seen_date.cmp(&b.metadata.first_seen_date))
            }
            SortType::DateModified => {
                v.sort_by(|a, b| a.metadata.modified_date.cmp(&b.metadata.modified_date))
            }
            SortType::DateCreated => {
                v.sort_by(|a, b| a.metadata.creation_date.cmp(&b.metadata.creation_date))
            }
            SortType::TagCount => v.sort_by(|a, b| a.tags.len().cmp(&b.tags.len())),
            SortType::Type => v.sort_by(|a, b| a.metadata.format.cmp(&b.metadata.format)),
            SortType::Size => v.sort_by(|a, b| a.metadata.filesize.cmp(&b.metadata.filesize)),
            SortType::Resolution => {
                v.sort_by(|a, b| a.metadata.resolution.cmp(&b.metadata.resolution))
            }

            // this one is different b/c it relies on a sort specific to videos.
            //
            // so we sort the types independently to ensure that photos are
            // also useful when a user finds them.
            SortType::Duration => {
                // create a list of all videos
                let mut videos = Vec::new();
                let mut photos = Vec::new();

                // move the entirety of `v` into a new vec
                let vec: Vec<Media> = mem::take(v);

                // split the vec into photos and videos
                for media in vec.into_iter() {
                    match media.metadata.specific {
                        SpecificMetadata::Image {} => photos.push(media),
                        SpecificMetadata::Video { length } => videos.push((media, length)),
                        _ => unreachable!("animated images aren't yet distinct from photos"),
                    }
                }

                // sort the videos by their duration
                videos.sort_by(|(_, a_len), (_, b_len)| a_len.total_cmp(b_len));

                // always sort photos by the creation date (this sucks but whatever)
                photos.sort_by(|a, b| a.metadata.creation_date.cmp(&b.metadata.creation_date));

                #[cfg(debug_assertions)]
                assert!(v.is_empty(), "the original vec should still be empty here");
                let total_len = videos.len() + photos.len();

                // and add everything back into `v`
                for (vid, _len) in videos {
                    v.push(vid);
                }

                for photo in photos {
                    v.insert(0, photo);
                }

                #[cfg(debug_assertions)]
                assert_eq!(
                    total_len,
                    v.len(),
                    "the sorted vec should have all original elements"
                );
            }
        }

        // if we're not doing a random sort, reverse the order when we're descending
        if ty != SortType::Random {
            if let SortOrder::Descending = order {
                v.reverse();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, time::SystemTime};

    use crate::models::metadata::{
        types::{Filesize, Format, Resolution},
        Metadata,
    };

    use super::*;

    #[tokio::test]
    async fn sort_by_size() {
        let mut v: Vec<Media> = Vec::new();
        for i in 0..10 {
            v.push(Media {
                metadata: {
                    let mut m = create_default_metadata();
                    m.filesize = Filesize(i as u64 * 1024);
                    m
                },
                tags: vec![],
            });
        }

        let mut query = FinishedQuery(v.clone());

        while query.0 == v {
            query.sort(SortType::Random, SortOrder::Ascending).await;
        }
        assert_ne!(query.0, v);

        query.sort(SortType::Size, SortOrder::Ascending).await;
        assert_eq!(query.0, v);
    }

    #[tokio::test]
    async fn sort_by_duration() {
        let mut v: Vec<Media> = Vec::new();

        v.push(Media {
            metadata: {
                let mut m = create_default_metadata();
                m.filesize = Filesize(2_000_000);
                m
            },
            tags: vec![],
        });

        for len in 1..=10 {
            v.push(Media {
                metadata: {
                    let mut m = create_default_metadata();
                    m.specific = SpecificMetadata::Video { length: len as f64 };
                    m.filesize = Filesize(len as u64 * 1024);
                    m
                },
                tags: vec![],
            });
        }

        let mut query = FinishedQuery(v.clone());
        query.sort(SortType::Duration, SortOrder::Descending).await;

        trait F {
            fn get_length(&self) -> f64;
        }

        impl F for Media {
            fn get_length(&self) -> f64 {
                if let SpecificMetadata::Video { length } = self.metadata.specific.clone() {
                    length
                } else {
                    0_f64
                }
            }
        }

        assert_eq!(
            query.0.iter().map(|m| m.get_length()).collect::<Vec<_>>(),
            v.iter().rev().map(|m| m.get_length()).collect::<Vec<_>>(),
            "descending"
        );

        // let's do another one, but ascending!
        query.sort(SortType::Duration, SortOrder::Ascending).await;

        assert_eq!(
            query.0.iter().map(|m| m.get_length()).collect::<Vec<_>>(),
            v.iter().map(|m| m.get_length()).collect::<Vec<_>>(),
            "ascending"
        );
    }

    fn create_default_metadata() -> Metadata {
        Metadata {
            path: PathBuf::from("a"),
            resolution: Resolution::new(1920, 1080),
            filesize: Filesize(1024),
            format: Format::new_from_mime("image/jpeg").unwrap(),
            creation_date: None,
            modified_date: None,
            first_seen_date: SystemTime::now(),
            specific: SpecificMetadata::Image {},
            other: None,
        }
    }
}
