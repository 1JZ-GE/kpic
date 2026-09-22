# kpic

imple image compressor for KDE Plasma 6

<img width="256" alt="image" src="https://github.com/user-attachments/assets/f824f208-d3b0-47b5-b4fd-122132678ee0" />

## Architecture

The widget is the UI. On load it spawns the bundled daemon (`contents/daemon/kpic`), which owns the session bus name `org.kpic.ImgSqueeze`. The widget calls `StartJob(uris, quality, lossless, format) -> jobId` and `Cancel(jobId)`, and receives `ProgressChanged` / `JobFinished` signals.

## Compression pipeline

`src/lib.rs` dispatches on input format and the UI's lossless flag:

| Input | Lossless | Engine |
|---|---|---|
| PNG | yes | oxipng (keeps original if not smaller) |
| PNG | no | `libimagequant` quantization + hand-rolled palette-PNG writer |
| JPG/JPEG | – | `image` crate re-encode at given quality |
| WebP | – | `webp` crate re-encode at given quality |

**Components:**

- **oxipng**
- **imagequant**
- **image** 
- **webp** 
- **flate2** 
- **zbus**
- **tokio**
- **serde** 

## Install

```sh
./scripts/install.sh
```

Installs the widget (`org.kpic.compressor`) with the compiled daemon bundled inside it (`contents/daemon/kpic`).

## Credits

thanks to luisbocanegra for reference and fix
and all author of the components being used
