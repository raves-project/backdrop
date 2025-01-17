/*! # `backdrop`

A library crate for the [Raves](https://github.com/onkoe/raves) Gallery app on Android.

## Purpose

This is the backend of Raves. It manages your media, helping you sort, search, organize, and use all of it in many unique ways.

Internally, `backdrop` is based on a database of collected/created metadata cached for each piece of media.

## Building

To build this, there are a few dependencies you need to install. I use Fedora, but please feel free to submit PRs to add package lists for other distributions.

### Fedora

`sudo dnf install -y nasm libgexiv2-devel libdav1d libdav1d-devel`

## Status

Under active development.

- [ ] GOAL: Feature-completeness
    - [ ] Metadata scanning for `Media`
        - [ ] Images
        - [ ] GIFS
        - [ ] Video
        - [ ] General (including Folder. i.e. `stat`)
    - [ ] Tagging
        - [ ] Can access existing tags from media (requires metadata)
        - [ ] Store in database using own format
            - Issueify but: entire database of serialized `Tag`s and `Media`.
        - [ ] Export database from own format to associate directly with media
        - [ ] Implied tags
            - Issueify but: "implied" means that media with one tag is implied to have another.
            - If it shouldn't have that tag, you can say that.
        - [ ] Associated people
            - If we go the route of having People (i.e. machine learning), we should be able to associate folks with their tags.
            - If a person is named "Barrett", allow users to associate them with the "barrett" tag (or any other).
                - for UI: warn on low overlap.
        - [ ] Recommended people tags
            - When they know someone is often involved with other tags, a user can add tags to show up by default.
            - ex: Dad has "family", "home", "overweight", etc.
    - [ ] Search
        - You should be able to search the database for virtually anything.
    - [ ] Cleanup
        - [ ] Image similarity
        - [ ] Tagging
    - [ ] Media operations queue
        - Issueify: Implement a "queue" of operations to perform on the data. Create `Future`s for each operation and lock affected media from operations until they are no longer used.
            - Locked media should only have some attributes locked, if even necessary at all. (i.e. the queue isn't running multiple things at once)
            - How does this affect search/navigation?

## Usage

You'll want to do three things when using this library in the app:

1. Setup logging to see the library's messages. (`tracing_subscriber`)
1. Make or load a configuration for the library. (`config::CONFIG`)
2. Use `database::DB_FOLDER_PATH.set(<path>)` to say where the database is (or will be) located.
3. Start the file watcher with `Watch::watch()`.

It's important that these tasks are performed **before** using the library. Otherwise, the backend will not be correctly initialized, and bugs may result.
*/

pub mod config;
pub mod database;
pub mod error;
pub mod models;
pub mod search;
pub mod watch;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
