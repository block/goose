# Goslingy

Put `goslingy` in your $PATH if you want to launch via:

```
goslingy .
```

This will open gosling GUI from any path you specify

# Unregister Deeplink Protocols (macos only)

`unregister-deeplink-protocols.js` is a script to unregister the deeplink protocol used by gosling like `gosling://`.
This is handy when you want to test deeplinks with the development version of Gosling.

# Usage

To unregister the deeplink protocols, run the following command in your terminal:
Then launch Gosling again and your deeplinks should work from the latest launched gosling application as it is registered on startup.

```bash
node scripts/unregister-deeplink-protocols.js
```

