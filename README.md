# imgsqueeze

Image compressor for KDE Plasma 6. Session D-Bus daemon plus a Plasma 6
widget (`org.kpic.compressor`).

## architecture

The widget is the UI. On load it spawns the bundled daemon
(`contents/daemon/imgsqueeze`), which owns session bus name
`org.kpic.ImgSqueeze`. The widget calls methods `StartJob(uris, quality,
lossless, format) -> jobId` and `Cancel(jobId)`, receiving `ProgressChanged`
and `JobFinished` signals. No D-Bus activation file, no manual daemon
management — same pattern as panel-colorizer: backend ships inside the
widget package, the widget starts it.

## install

```
./scripts/install.sh
```

One command installs the widget (`org.kpic.compressor`) with the compiled
daemon bundled inside it (`contents/daemon/imgsqueeze`).

For development, point `plasmoidviewer -a kpic/plasmoid/package` straight at
the package directory; run the daemon manually with
`kpic/target/release/imgsqueeze`.

## tests

```
cargo test
```