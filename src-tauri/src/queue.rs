//! Sequential queue shared by the desktop worker and regression tests.
use crate::{
    model::{Task, TaskRequest},
    runner, store,
};
use std::{
    collections::{HashMap, VecDeque},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};
pub struct Pending {
    pub id: String,
    pub request: TaskRequest,
    pub cancel: Arc<AtomicBool>,
}
#[derive(Default)]
pub struct Queue {
    pub stopping: bool,
    pub pending: VecDeque<Pending>,
    pub flags: HashMap<String, Arc<AtomicBool>>,
    pub installing: bool,
}
impl Queue {
    pub fn enqueue(&mut self, db: &Path, request: TaskRequest) -> Result<(), String> {
        if self.stopping {
            return Err("L’application est en cours de fermeture.".into());
        }
        if self.installing {
            return Err("Attendez la fin de l’installation du moteur.".into());
        }
        if request.inputs.len() > 200 {
            return Err("Choisissez au maximum 200 fichiers.".into());
        }
        let requests = if request.operation == "pdf.merge" {
            vec![request]
        } else {
            request
                .inputs
                .iter()
                .map(|p| {
                    let mut item = request.clone();
                    item.inputs = vec![p.clone()];
                    item
                })
                .collect::<Vec<_>>()
        };
        if requests.is_empty() {
            return Err("Choisissez des fichiers.".into());
        }
        if self.flags.len() + requests.len() > 200 {
            return Err("La file est pleine (200 tâches maximum).".into());
        }
        for request in &requests {
            runner::validate(request)?;
        }
        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs() as i64;
        let items: Vec<_> = requests
            .into_iter()
            .map(|request| Pending {
                id: uuid::Uuid::new_v4().to_string(),
                request,
                cancel: Arc::new(AtomicBool::new(false)),
            })
            .collect();
        // One SQLite transaction: a batch is either fully recorded or not enqueued.
        let tasks: Vec<_> = items
            .iter()
            .map(|p| Task {
                id: p.id.clone(),
                operation: p.request.operation.clone(),
                status: "queued".into(),
                created,
                output: None,
                error: None,
            })
            .collect();
        store::insert_batch(db, &tasks)?;
        for item in items {
            self.flags.insert(item.id.clone(), item.cancel.clone());
            self.pending.push_back(item);
        }
        Ok(())
    }
    pub fn cancel(&self, id: &str) -> Result<(), String> {
        self.flags
            .get(id)
            .ok_or("Cette tâche est déjà terminée.")?
            .store(true, Ordering::SeqCst);
        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), String> {
        if self.installing {
            return Err("Attendez la fin de l’installation avant de quitter.".into());
        }
        self.stopping = true;
        for flag in self.flags.values() {
            flag.store(true, Ordering::SeqCst);
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, std::path::PathBuf, TaskRequest) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("history.db");
        store::init(&db).unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        std::fs::write(&a, b"a").unwrap();
        std::fs::write(&b, b"b").unwrap();
        let req = TaskRequest {
            operation: "audio.convert".into(),
            inputs: vec![a.to_str().unwrap().into(), b.to_str().unwrap().into()],
            destination: dir.path().to_str().unwrap().into(),
            format: "flac".into(),
            pages: "1".into(),
            width: 32,
            quality: 23,
            track: 0,
        };
        (dir, db, req)
    }
    #[test]
    fn regression_batch_is_fifo_and_cancel_does_not_cancel_next() {
        let (_dir, db, r) = fixture();
        let mut q = Queue::default();
        q.enqueue(&db, r.clone()).unwrap();
        let first = q.pending.pop_front().unwrap();
        q.cancel(&first.id).unwrap();
        let second = q.pending.pop_front().unwrap();
        assert_eq!(first.request.inputs, [r.inputs[0].clone()]);
        assert_eq!(second.request.inputs, [r.inputs[1].clone()]);
        assert!(first.cancel.load(Ordering::SeqCst));
        assert!(!second.cancel.load(Ordering::SeqCst));
        assert_eq!(store::list(&db).unwrap().len(), 2);
    }
    #[test]
    fn regression_invalid_batch_is_atomic() {
        let (_dir, db, mut r) = fixture();
        r.inputs.push("/does-not-exist.wav".into());
        let mut q = Queue::default();
        assert!(q.enqueue(&db, r).is_err());
        assert!(q.pending.is_empty());
        assert!(q.flags.is_empty());
        assert!(store::list(&db).unwrap().is_empty());
    }
    #[test]
    fn regression_database_failure_does_not_enqueue_partial_batch() {
        let (dir, db, r) = fixture();
        store::connect(&db).unwrap().execute_batch("CREATE TRIGGER reject_second BEFORE INSERT ON tasks WHEN (SELECT COUNT(*) FROM tasks)>0 BEGIN SELECT RAISE(ABORT,'disk write failure'); END;").unwrap();
        let mut q = Queue::default();
        assert!(q.enqueue(&db, r).is_err());
        assert!(q.pending.is_empty());
        assert!(store::list(&db).unwrap().is_empty());
        assert!(dir.path().is_dir());
    }
    #[test]
    fn regression_shutdown_cancels_all_and_rejects_new_jobs() {
        let (_dir, db, r) = fixture();
        let mut q = Queue::default();
        q.enqueue(&db, r.clone()).unwrap();
        q.stop().unwrap();
        assert!(q.flags.values().all(|f| f.load(Ordering::SeqCst)));
        assert!(q.enqueue(&db, r).is_err());
    }
    #[test]
    fn regression_installation_excludes_new_jobs_and_shutdown() {
        let (_dir, db, r) = fixture();
        let mut q = Queue {
            installing: true,
            ..Queue::default()
        };
        assert!(q.enqueue(&db, r).is_err());
        assert!(q.stop().is_err());
        assert!(!q.stopping);
    }
    #[test]
    fn regression_pending_queue_limit_is_enforced_without_partial_batch() {
        let (_dir, db, r) = fixture();
        let mut q = Queue::default();
        for _ in 0..100 {
            q.enqueue(&db, r.clone()).unwrap();
        }
        assert_eq!(q.flags.len(), 200);
        assert!(q.enqueue(&db, r).is_err());
        assert_eq!(store::list(&db).unwrap().len(), 200);
    }
}
