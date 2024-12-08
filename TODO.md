# quality
- [ ] dangling commits or sth?! (spurious artwork write ops)
- [ ] use string PKs?

# features
- [ ] album art: support 'folder' file
- [ ] transcoding
- [ ] web UI
- [ ] accounts/admin
- [ ] fswatch (notify-rs)
    - [ ] PollWatcher?
- [ ] hash data not metadata so tags can be edited but we don't lose index
    - [ ] manage missing files
        - [ ] auto backup DB for undo

## dedup
- content adressable
    - cover art
    - songs?

## quality/safety
- [ ] DB indexing
- [ ] DB transactions
- [ ] audit all unwrap/expect

## dox
### dev
- `sea-orm-cli migrate generate -d src/indexer/migration`
- FE: tailwind watcher