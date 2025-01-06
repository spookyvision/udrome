# udrome frontend

minimal docs for now; unix-y environments only

## Setup
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
The backend is supposed to serve the frontend static files from `$DATA_DIR/public`.
If you use a reverse proxy, set your base url in `Dioxus.toml`:
```toml
base_path = "udrome"
```

```bash
./_tailwind # compile tailwind once
dx bundle # build static site
cp -r target/dx/udrome-frontend/release/web/public $DATA_DIR
```