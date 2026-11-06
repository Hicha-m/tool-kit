use crate::model::Task;
use rusqlite::{params, Connection};
use std::path::Path;
pub fn connect(path:&Path)->Result<Connection,String> {
 let db=Connection::open(path).map_err(|e|e.to_string())?;
 db.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e|e.to_string())?;Ok(db)
}
pub fn init(path:&Path)->Result<(),String> {
 let db=connect(path)?;
 db.execute_batch("CREATE TABLE IF NOT EXISTS temporary_outputs(path TEXT PRIMARY KEY); CREATE TABLE IF NOT EXISTS tasks(id TEXT PRIMARY KEY, operation TEXT NOT NULL, status TEXT NOT NULL, created INTEGER NOT NULL, output TEXT, error TEXT); UPDATE tasks SET status='interrupted', error='L’application a été arrêtée avant la fin.' WHERE status IN ('queued','running'); PRAGMA user_version=1;").map_err(|e|e.to_string())?;
 let paths={let mut stmt=db.prepare("SELECT path FROM temporary_outputs").map_err(|e|e.to_string())?;let rows=stmt.query_map([],|row|row.get::<_,String>(0)).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
 for path in paths {let p=Path::new(&path);if p.file_name().map(|n|n.to_string_lossy().starts_with(".toolkit-")).unwrap_or(false){match std::fs::remove_file(p){Ok(())=>{},Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{},Err(_)=>continue}}
 db.execute("DELETE FROM temporary_outputs WHERE path=?1",[path]).map_err(|e|e.to_string())?;}
 Ok(())
}
pub fn insert(path:&Path, t:&Task)->Result<(),String> {
 connect(path)?.execute("INSERT INTO tasks VALUES (?1,?2,?3,?4,?5,?6)",params![t.id,t.operation,t.status,t.created,t.output,t.error]).map_err(|e|e.to_string())?;Ok(())
}
pub fn update(path:&Path,id:&str,status:&str,output:Option<&str>,error:Option<&str>)->Result<(),String> {
 let db=connect(path)?;
 db.execute("UPDATE tasks SET status=?2,output=?3,error=?4 WHERE id=?1",params![id,status,output,error]).map_err(|e|e.to_string())?;
 db.execute("DELETE FROM tasks WHERE status NOT IN ('queued','running') AND id NOT IN (SELECT id FROM tasks WHERE status NOT IN ('queued','running') ORDER BY created DESC,rowid DESC LIMIT 100)",[]).map_err(|e|e.to_string())?;Ok(())
}
pub fn list(path:&Path)->Result<Vec<Task>,String> {
 let db=connect(path)?;
 let mut stmt=db.prepare("SELECT id,operation,status,created,output,error FROM tasks ORDER BY created DESC,rowid DESC").map_err(|e|e.to_string())?;
 let rows=stmt.query_map([],|r|Ok(Task{id:r.get(0)?,operation:r.get(1)?,status:r.get(2)?,created:r.get(3)?,output:r.get(4)?,error:r.get(5)?})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}

#[cfg(test)] mod tests {
 use super::*;
 #[test] fn restart_interrupts_tasks_and_cleans_only_recorded_temporary_output(){
  let dir=tempfile::tempdir().unwrap();let db=dir.path().join("test.sqlite3");init(&db).unwrap();
  let output=dir.path().join(".toolkit-interrupted.png");std::fs::write(&output,b"partial").unwrap();
  let source=dir.path().join("source.png");std::fs::write(&source,b"original").unwrap();
  connect(&db).unwrap().execute("INSERT INTO temporary_outputs VALUES (?1)",[output.to_str().unwrap()]).unwrap();
  insert(&db,&Task{id:"test".into(),operation:"image.convert".into(),status:"running".into(),created:1,output:None,error:None}).unwrap();
  init(&db).unwrap();assert_eq!(list(&db).unwrap()[0].status,"interrupted");assert!(!output.exists());assert_eq!(std::fs::read(source).unwrap(),b"original");
 }
 #[test] fn history_keeps_active_jobs_and_only_100_finished_results(){
  let dir=tempfile::tempdir().unwrap();let db=dir.path().join("test.sqlite3");init(&db).unwrap();
  for i in 0..105 {let id=i.to_string();insert(&db,&Task{id:id.clone(),operation:"pdf.merge".into(),status:"queued".into(),created:i,output:None,error:None}).unwrap();update(&db,&id,"succeeded",Some("result.pdf"),None).unwrap();}
  insert(&db,&Task{id:"pending".into(),operation:"pdf.merge".into(),status:"queued".into(),created:106,output:None,error:None}).unwrap();
  let jobs=list(&db).unwrap();assert_eq!(jobs.len(),101);assert_eq!(jobs[0].status,"queued");
 }
}
