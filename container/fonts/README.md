# Optional Windows fonts

Place properly licensed `.ttf`, `.ttc`, or `.otf` files in this directory before building the
image, or mount them here at runtime:

```text
/usr/local/share/fonts/windows
```

Recommended families for the current Windows desktop profile include Segoe UI, Arial, Calibri,
Cambria, Consolas, Times New Roman, Courier New, Tahoma, Verdana, Georgia, and Trebuchet MS.

The font binaries are ignored by Git. The container entrypoint refreshes the user fontconfig cache
so fonts supplied through a Kubernetes volume are available to Chromium.
