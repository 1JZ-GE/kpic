// session d-bus daemon: compress batches with live progress + results.
use kpic::{run_batch, BatchProgress, CompressOptions, FileResult};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tokio::task;
use zbus::{connection, interface, Connection};

const BUS_NAME: &str = "org.kpic.ImgSqueeze";
const PATH: &str = "/org/kpic/ImgSqueeze";
const IFACE: &str = "org.kpic.ImgSqueeze";

struct SearchTag {
    cancel: Arc<AtomicBool>,
    files: Vec<PathBuf>,
    opts: CompressOptions,
}

pub struct ImgSqueeze {
    conn: Connection,
    jobs: Arc<Mutex<HashMap<u32, Arc<SearchTag>>>>,
    next_id: AtomicU32,
}

impl ImgSqueeze {
    fn new(conn: Connection) -> Self {
        Self {
            conn,
            jobs: Arc::new(Mutex::new(HashMap::new())),
            next_id: AtomicU32::new(1),
        }
    }
}

#[interface(name = "org.kpic.ImgSqueeze")]
impl ImgSqueeze {
    async fn start_job(
        &self,
        uris: Vec<String>,
        quality: u8,
        lossless: bool,
        format: String,
    ) -> zbus::fdo::Result<u32> {
        if uris.is_empty() {
            return Ok(0);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let tag = Arc::new(SearchTag {
            cancel: Arc::new(AtomicBool::new(false)),
            // drop uris come as file:// links, not plain paths
            files: uris
                .iter()
                .map(|u| PathBuf::from(u.strip_prefix("file://").unwrap_or(u)))
                .collect(),
            opts: CompressOptions {
                quality,
                lossless,
                format: if format.is_empty() { None } else { Some(format) },
            },
        });
        self.jobs.lock().unwrap().insert(id, tag.clone());

        let conn = self.conn.clone();
        let jobs = self.jobs.clone();
        task::spawn(async move {
            let total = tag.files.len();
            let (tx, rx) = mpsc::channel::<(u32, usize)>();
            let progress_tx = tx.clone();
            let emit_conn = conn.clone();
            let emit_task = task::spawn(async move {
                while let Ok((job_id, done)) = rx.recv() {
                    if let Err(e) = emit_conn
                        .emit_signal(
                            None::<&str>,
                            PATH,
                            IFACE,
                            "ProgressChanged",
                            &(job_id, done as u32, done as u64, total as u64, String::new()),
                        )
                        .await
                    {
                        eprintln!("emit ProgressChanged failed: {e}");
                    }
                }
            });
            let progress = BatchProgress {
                callback: Box::new(move |done| {
                    let _ = progress_tx.send((id, done));
                }),
            };
            // drop the original sender so the channel closes when progress goes.
            drop(tx);
            let results = run_batch(&tag.files, &tag.opts, Some(&tag.cancel), Some(&progress));
            // close the channel so the emit task drains and exits.
            drop(progress);
            let _ = emit_task.await;
            let (uris, outputs, ins, outs, errors) = file_result_arrays(&results);
            if let Err(e) = conn
                .emit_signal(
                    None::<&str>,
                    PATH,
                    IFACE,
                    "JobFinished",
                    &(id, uris, outputs, ins, outs, errors),
                )
                .await
            {
                eprintln!("emit JobFinished failed: {e}");
            }
            jobs.lock().unwrap().remove(&id);
        });
        Ok(id)
    }

    /// cancel a running job; the in-flight file finishes, the rest are skipped.
    async fn cancel(&self, job_id: u32) {
        if let Some(tag) = self.jobs.lock().unwrap().get(&job_id) {
            tag.cancel.store(true, Ordering::Relaxed);
        }
    }
}

type FileResultArrays = (Vec<String>, Vec<String>, Vec<u64>, Vec<u64>, Vec<String>);

fn file_result_arrays(results: &[FileResult]) -> FileResultArrays {
    results.iter().fold(
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()),
        |(mut u, mut o, mut i, mut os, mut e), r| {
            u.push(r.input.display().to_string());
            o.push(r.output.as_ref().map(|p| p.display().to_string()).unwrap_or_default());
            i.push(r.input_size);
            os.push(r.output_size.unwrap_or(0));
            e.push(r.error.clone().unwrap_or_default());
            (u, o, i, os, e)
        },
    )
}

pub async fn run() -> zbus::Result<()> {
    let conn = connection::Builder::session()?.name(BUS_NAME)?.build().await?;
    conn.object_server().at(PATH, ImgSqueeze::new(conn.clone())).await?;
    std::future::pending::<()>().await;
    Ok(())
}

/// entrypoint for the sync cli; spins up the tokio runtime.
pub fn run_blocking() -> zbus::Result<()> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(run())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_flag_skips_remaining_files() {
        let cancel = std::sync::atomic::AtomicBool::new(true);
        let opts = CompressOptions { quality: 80, lossless: false, format: Some("png".into()) };
        // daemon maps skipped res to error entries; run_batch already guarantees 2 entries
        let res = run_batch(&["a.png".into(), "b.png".into()], &opts, Some(&cancel), None);
        assert_eq!(res.len(), 2);
        assert!(res[1].error.is_some());
    }

    #[test]
    fn file_result_to_dbus_arrays_maps_fields() {
        let fr = FileResult {
            input: "a.png".into(),
            output: Some("a-compressed.png".into()),
            input_size: 10,
            output_size: Some(5),
            error: None,
        };
        let (uris, outputs, ins, outs, errors) = file_result_arrays(&[fr]);
        assert_eq!(uris, vec!["a.png".to_string()]);
        assert_eq!(outputs, vec!["a-compressed.png".to_string()]);
        assert_eq!(ins, vec![10]);
        assert_eq!(outs, vec![5]);
        assert_eq!(errors, vec!["".to_string()]);
    }
}