#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod engines;mod model;mod runner;mod store;
use model::{Engine,Task,TaskRequest};
use std::{collections::{VecDeque,HashMap},path::PathBuf,sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}},thread,time::{Duration,SystemTime,UNIX_EPOCH}};
use tauri::{Emitter,Manager,State};
struct Pending {id:String,request:TaskRequest,cancel:Arc<AtomicBool>}
#[derive(Default)] struct Queue {stopping:bool,pending:VecDeque<Pending>, flags:HashMap<String,Arc<AtomicBool>>, installing:bool}
struct AppState {data:PathBuf,db:PathBuf,queue:Arc<Mutex<Queue>>}
#[tauri::command] fn list_tasks(state:State<AppState>)->Result<Vec<Task>,String>{store::list(&state.db)}
#[tauri::command] fn list_modules()->Result<serde_json::Value,String>{serde_json::from_str(include_str!("../catalog/modules.json")).map_err(|e|e.to_string())}
#[tauri::command] fn list_engines(state:State<AppState>)->Result<Vec<Engine>,String>{engines::list(&state.data)}
#[tauri::command] fn enqueue_task(state:State<AppState>,request:TaskRequest)->Result<(),String>{
 let mut queue=state.queue.lock().map_err(|_|"File indisponible.")?;
 if queue.stopping{return Err("L’application est en cours de fermeture.".into())}
 if queue.installing{return Err("Attendez la fin de l’installation du moteur.".into())}
 if queue.flags.len()+request.inputs.len()>200{return Err("La file est pleine (200 fichiers maximum).".into())}
 engines::executable(&state.data,model::engine_for(&request.operation)?,false)?;
 let requests=if request.operation=="pdf.merge"{vec![request]}else{request.inputs.iter().map(|p|{let mut item=request.clone();item.inputs=vec![p.clone()];item}).collect::<Vec<_>>()};
 if requests.is_empty(){return Err("Choisissez des fichiers.".into())}
 for r in &requests{runner::validate(r)?;}
 for r in requests {
 let id=uuid::Uuid::new_v4().to_string();let flag=Arc::new(AtomicBool::new(false));
 let task=Task{id:id.clone(),operation:r.operation.clone(),status:"queued".into(),created:SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_secs() as i64,output:None,error:None};
 store::insert(&state.db,&task)?;queue.flags.insert(id.clone(),flag.clone());queue.pending.push_back(Pending{id,request:r,cancel:flag});
 }Ok(())
}
#[tauri::command] fn cancel_task(state:State<AppState>,id:String)->Result<(),String>{
 let queue=state.queue.lock().map_err(|_|"File indisponible.")?;
 let flag=queue.flags.get(&id).ok_or("Cette tâche est déjà terminée.")?;flag.store(true,Ordering::SeqCst);Ok(())
}
#[tauri::command] fn clear_history(state:State<AppState>)->Result<(),String>{store::connect(&state.db)?.execute("DELETE FROM tasks WHERE status NOT IN ('queued','running')",[]).map_err(|e|e.to_string())?;Ok(())}
#[tauri::command] fn open_result(state:State<AppState>,path:String)->Result<(),String>{
 let allowed=store::list(&state.db)?.iter().any(|t|t.status=="succeeded"&&t.output.as_deref()==Some(path.as_str()));
 if !allowed||!std::path::Path::new(&path).is_file(){return Err("Le résultat n’existe plus ou n’appartient pas à l’historique.".into())}
 open::that_detached(path).map_err(|e|e.to_string())
}
#[tauri::command] async fn stop_all(state:State<'_,AppState>)->Result<(),String>{
 let queue=state.queue.clone();
 {let mut q=queue.lock().map_err(|_|"File indisponible.")?;if q.installing{return Err("Attendez la fin de l’installation avant de quitter.".into())}q.stopping=true;for flag in q.flags.values(){flag.store(true,Ordering::SeqCst);}}
 tauri::async_runtime::spawn_blocking(move||{loop{if queue.lock().map_err(|_|"File indisponible.")?.flags.is_empty(){return Ok(())}thread::sleep(Duration::from_millis(100));}}).await.map_err(|e|e.to_string())?
}
#[tauri::command] async fn install_engine(state:State<'_,AppState>,id:String)->Result<(),String>{
 let data=state.data.clone();let queue=state.queue.clone();
 {let mut q=queue.lock().map_err(|_|"File indisponible.")?;if q.stopping||q.installing||!q.flags.is_empty(){return Err("Attendez que la file soit vide avant l’installation.".into())}q.installing=true;}
 let result=tauri::async_runtime::spawn_blocking(move||engines::install(&data,&id)).await.map_err(|e|e.to_string()).and_then(|r|r);
 if let Ok(mut q)=queue.lock(){q.installing=false;}result
}
fn main(){
 tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app,_,_|{if let Some(w)=app.get_webview_window("main"){let _=w.set_focus();}})).plugin(tauri_plugin_dialog::init()).setup(|app|{
 let data=app.path().app_data_dir()?;std::fs::create_dir_all(&data)?;let db=data.join("history.sqlite3");store::init(&db)?;
 let queue=Arc::new(Mutex::new(Queue::default()));let worker_queue=queue.clone();let worker_data=data.clone();let worker_db=db.clone();let handle=app.handle().clone();
 app.manage(AppState{data,db,queue});
 thread::spawn(move||loop{
 let item=match worker_queue.lock(){Ok(mut q)=>q.pending.pop_front(),Err(_)=>return};
 if let Some(item)=item {
 let result=if item.cancel.load(Ordering::SeqCst){Err("Annulé".into())}else{
 if let Err(e)=store::update(&worker_db,&item.id,"running",None,None){Err(e)}else{let _=handle.emit("tasks-changed",());runner::run(&worker_data,&worker_db,&item.request,&item.cancel)}
 };
 let status=if result.is_ok(){"succeeded"}else if item.cancel.load(Ordering::SeqCst){"cancelled"}else{"failed"};
 let (output,error)=match result{Ok(p)=>(Some(p.to_string_lossy().into_owned()),None),Err(e)=>(None,Some(e))};
 if let Err(e)=store::update(&worker_db,&item.id,status,output.as_deref(),error.as_deref()){eprintln!("Impossible de mettre à jour l’historique : {e}");}
 if let Ok(mut q)=worker_queue.lock(){q.flags.remove(&item.id);}let _=handle.emit("tasks-changed",());
 }else{thread::sleep(Duration::from_millis(150));}
 });Ok(())
 }).invoke_handler(tauri::generate_handler![list_tasks,list_modules,list_engines,enqueue_task,cancel_task,clear_history,open_result,install_engine,stop_all])
 .run(tauri::generate_context!()).expect("Impossible de démarrer Tool-kit");
}
