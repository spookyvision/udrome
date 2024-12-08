# Setup
```bash
npm install
```

# Development

```bash
./watch_tailwind &
dx serve
```

# Bundle ("release build")

```bash
./_tailwind
dx bundle
cp -r target/dx/udrome-frontend/release/web/public $DATA_DIR
```

### Tailwind
```bash
fswatch . | xargs -n 1 ./watchee
```

### Serving Your App

Run the following command in the root of your project to start developing with the default platform:

```bash
dx serve
```

To run for a different platform, use the `--platform platform` flag. E.g.
```bash
dx serve --platform desktop
```

