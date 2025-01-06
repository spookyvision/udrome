# quality/safety
- [ ] clean up base_url, no leading/trailing slash
- [ ] dangling DB commits or sth?! (spurious artwork write ops)
- [ ] use string PKs?
- [ ] DB transactions
- [ ] audit all unwrap/expect

# features
## backend
- [ ] album art: support 'folder' file
- [ ] transcoding
- [ ] accounts/admin
- [ ] fswatch (notify-rs)
    - [ ] PollWatcher?
- [ ] hash data not metadata so tags can be edited but we don't lose index
    - [ ] manage missing files
        - [ ] auto backup DB for undo

## frontend
- [ ] mobile layout
- [ ] i18n
- [ ] a11y
- [ ] desktop app

## dedup
- content adressable
    - cover art
    - songs?


## dox
- [ ] manual
### dev
- `sea-orm-cli migrate generate -d src/indexer/migration`
- FE: tailwind watcher