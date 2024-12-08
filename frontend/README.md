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

## Bundle ("release build")
the backend is supposed to serve the frontend static files from `$DATA_DIR/public`.

```bash
./_tailwind # compile tailwind once
dx bundle # build static site
cp -r target/dx/udrome-frontend/release/web/public $DATA_DIR
```