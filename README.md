<!-- cargo-rdme start -->

# `backdrop`

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
  - [ ] Search
    - You should be able to search the database for virtually anything.
  - [ ] Cleanup
    - [ ] Image similarity
    - [ ] Tagging
  - [ ] Media operations queue
    - Issueify: Implement a "queue" of operations to perform on the data. Create `Future`s for each operation and lock affected media from operations until they are no longer used.
      - Locked media should only have some attributes locked, if even necessary at all. (i.e. the queue isn't running multiple things at once)
      - How does this affect search/navigation?

<!-- cargo-rdme end -->
