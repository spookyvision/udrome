# udrome
- ... because it's a very, *very* small clone of navidrome
- and it wasn't built in one day

## Pronounciation
- microdrome (metric)
- 2.54e-6drome (imperial and/or research chemist)
- ghoti (contrarian)

## Running
### Backend
The udrome server speaks (a minimal subset of) Subsonic. The only player I'm currently testing with is [Feishin](https://github.com/jeffvli/feishin) with udrome configured as Subsonic server (needs latest Feishin).

```bash
cp udrome.example.toml udrome.toml # and edit it
cargo run
```

### Frontend
udrome also ships with its own frontend/music player - see `frontend/README.md`