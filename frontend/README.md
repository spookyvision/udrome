# udrome frontend

minimal docs for now; unix-y environments only

## Setup
install `dioxus-cli v0.6.x` as per https://dioxuslabs.com/learn/0.6/getting_started/, then:
```bash
npm install
```

## Development
In development mode we're using a separate frontend server for hot reloading etc.
Setting `dev=true` in `udrome.toml` is required (it sets up CORS to allow any request).

```bash
./watch_tailwind & # compile tailwind in the background when things change
BACKEND_URL=http://localhost:3000 dx serve
```

### Hot reload and `tailwind.css`
The hot reload system skips `.gitignore`d files, which is a bit of a bummer. 
For that reason `assets/tailwind.css` is not in the ignore file, even though it is
generated and would pollute commits/history quite a bit. For that reason please don't
commit it - a suitable way to auto-ignore it is adding it to `.git/info/exclude`.

## Bundle ("release build")
The backend is supposed to serve the frontend static files from `$DATA_PATH/public`.
If you use a reverse proxy, set your base url in the backend `udrome.toml`
and also in `Dioxus.toml` as well, too:
```toml
[web.app]
base_path = "udrome"
```

```bash
./_tailwind # compile tailwind once
dx bundle # build static site

# read data_path value into $DATA_PATH. You can also (should, really) do this manually
DATA_PATH=$(awk '/data_path/ {gsub("\"","",$3); print $3}' ../udrome.toml) 

cp -r target/dx/udrome-frontend/release/web/public $DATA_PATH
```