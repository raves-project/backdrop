<!-- cargo-rdme start -->

 # `backdrop`

A library crate for the [Raves](https://github.com/onkoe/raves) Gallery app on Android.

## Purpose

This is the backend of Raves. It manages your media, helping you sort, search, organize, and use all of it in many unique ways.

Internally, `backdrop` is based on a database of collected/created metadata cached for each piece of media.

## Building

To build `backdrop`, you'll need to use Pixi. Grab `curl`, then follow [Pixi's installation instructions](https://pixi.sh/latest/installation/) to set it up (don't worry - it takes maybe 30 seconds). Afterward, clone the repo and run `pixi i && pixi shell`. You can now `cargo build`!

If you want to make changes, ensure you open your editor within the Pixi workspace: `pixi i && pixi shell && zed . --new`. Otherwise, system dependencies will be missing...

## Status

Under active development.

- [ ] GOAL: Feature-completeness
    - [x] Metadata scanning for `Media`
        - [x] Images
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

<!-- cargo-rdme end -->
