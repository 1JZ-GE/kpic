# kpic

Batch image compressor for KDE Plasma 6. Session D-Bus daemon plus a Plasma 6
widget (`org.kpic.compressor`).

## architecture

The widget is the UI. On load it spawns the bundled daemon
(`contents/daemon/kpic`), which owns session bus name `org.kpic.ImgSqueeze`.
The widget calls methods `StartJob(uris, quality, lossless, format) -> jobId`
and `Cancel(jobId)`, receiving `ProgressChanged` and `JobFinished` signals.
No D-Bus activation file, no manual daemon management — same pattern as
panel-colorizer: backend ships inside the widget package, the widget starts
it.

## compression pipeline

`src/lib.rs` dispatches on input format and the UI's lossless flag:

| input | lossless | engine |
|---|---|---|
| png | yes | oxipng (keep original if not smaller) |
| png | no | Rust libimagequant quantization + hand-rolled palette-PNG writer |
| jpg/jpeg | - | image crate re-encode at given quality |
| webp | - | webp crate re-encode at given quality |

Components used:

- **oxipng** (MIT, pure Rust) — lossless PNG optimization via `optimize_from_memory`, preset 2, `parallel` feature.
- **imagequant** (GPL-3.0-or-later, pure Rust) — lossy PNG via libimagequant 4.5.0, vendored as path dep `../pngquant/lib`. Quality mapped as `--quality=q-100`, speed 1, dithering 1.0.
- **image** (MIT, pure Rust) — JPEG decode/re-encode and WebP decode.
- **webp** (BSD/patent-cleared, pure Rust) — WebP encode.
- **flate2** (MIT/Apache-2.0, pure Rust) — deflate for the hand-rolled palette-PNG writer (own CRC32/chunk assembly).
- **zbus** + **tokio** (MIT/Apache-2.0) — session D-Bus daemon; **serde** for signal payloads.

The whole daemon is static, zero external image libraries (`ldd` clean),
and swap-free: pure Rust top to bottom.

Lossy PNG never writes an output larger than its input (skip-if-larger
guard, same as `--skip-if-larger` in the pngquant CLI). Lossless PNG is
always byte-identical pixels, and only kept when smaller.

License note: the daemon is MIT; the lossy-PNG path pulls in
imagequant (GPL-3.0-or-later). Acceptable for personal use, reconsider
before distribution.

## install

```
./scripts/install.sh
```

One command installs the widget (`org.kpic.compressor`) with the compiled
daemon bundled inside it (`contents/daemon/kpic`).

For development, point `plasmoidviewer -a kpic/plasmoid/package` straight at
the package directory; run the daemon manually with
`kpic/target/release/kpic`.

## tests

```
cargo test
```

Coverage: JPEG/WebP re-encode, lossy PNG quantization (palette output),
lossless PNG pixel bit-exactness + never-grows, skip-if-larger on lossy
paths.
