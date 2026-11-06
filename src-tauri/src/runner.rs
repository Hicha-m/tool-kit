use crate::{engines,model::{engine_for,pages_valid,TaskRequest}};
use command_group::CommandGroup;
use std::{fs,io::{Read,Seek,SeekFrom},path::{Path,PathBuf},process::{Command,Stdio},sync::{Arc,atomic::{AtomicBool,Ordering}},thread,time::Duration};

pub fn validate(r:&TaskRequest)->Result<(),String> {
 engine_for(&r.operation)?;
 if r.inputs.is_empty()||r.inputs.len()>200{return Err("Choisissez entre 1 et 200 fichiers.".into())}
 if r.operation=="pdf.merge"&&r.inputs.len()<2{return Err("Choisissez au moins deux PDF.".into())}
 if r.operation!="pdf.merge"&&r.inputs.len()!=1{return Err("Une tâche attend exactement un fichier.".into())}
 for p in &r.inputs {let p=Path::new(p);if !p.is_absolute()||!p.is_file(){return Err("Un fichier source est introuvable.".into())}}
 if !Path::new(&r.destination).is_absolute()||!Path::new(&r.destination).is_dir(){return Err("Choisissez un dossier de destination existant.".into())}
 let allowed=match r.operation.as_str(){"pdf.merge"|"pdf.extract"=>vec!["pdf"],"video.convert"=>vec!["mp4"],"image.convert"|"image.resize"=>vec!["jpeg","png","webp"],_=>vec!["mp3","wav","flac"]};
 if !allowed.contains(&r.format.as_str()){return Err("Format de sortie non pris en charge.".into())}
 if r.operation=="image.resize" && !(1..=16384).contains(&r.width){return Err("La largeur doit être comprise entre 1 et 16384 pixels.".into())}
 if r.operation=="video.convert" && !(18..=30).contains(&r.quality){return Err("La qualité CRF doit être comprise entre 18 et 30.".into())}Ok(())
}
// Separate process group: cancellation also terminates engine descendants.
fn execute(mut command:Command,cancel:&Arc<AtomicBool>,allow_warning:bool)->Result<String,String> {
 let mut output=tempfile::tempfile().map_err(|e|e.to_string())?;
 command.stdin(Stdio::null()).stdout(Stdio::from(output.try_clone().map_err(|e|e.to_string())?)).stderr(Stdio::null());
 let mut child=command.group_spawn().map_err(|_|"Le moteur n’a pas pu démarrer.")?;
 let status=loop {
 if cancel.load(Ordering::SeqCst){let _=child.kill();let _=child.wait();return Err("Annulé".into())}
 match child.try_wait(){Ok(Some(s))=>break s,Ok(None)=>thread::sleep(Duration::from_millis(80)),Err(_)=>{let _=child.kill();let _=child.wait();return Err("Impossible de suivre le processus.".into())}}
 };
 if !status.success() && !(allow_warning&&status.code()==Some(3)){return Err("Le moteur a refusé le fichier ou n’a pas pu produire le résultat. Vérifiez son format et l’espace disponible.".into())}
 output.seek(SeekFrom::Start(0)).map_err(|e|e.to_string())?;let mut text=String::new();output.take(65536).read_to_string(&mut text).map_err(|e|e.to_string())?;Ok(text)
}
fn cmd(bin:&Path,args:&[&str])->Command {let mut c=Command::new(bin);c.args(args);c}
pub fn run(data:&Path,db:&Path,r:&TaskRequest,cancel:&Arc<AtomicBool>)->Result<PathBuf,String> {
 validate(r)?;let engine=engine_for(&r.operation)?;let bin=engines::executable(data,engine,false)?;
 let dest=Path::new(&r.destination).canonicalize().map_err(|e|e.to_string())?;
 let temp=tempfile::Builder::new().prefix(".toolkit-").suffix(&format!(".{}",r.format)).tempfile_in(&dest).map_err(|_|"Impossible d’écrire dans le dossier de destination.")?;
 let recorded=temp.path().to_string_lossy().into_owned();
 crate::store::connect(db)?.execute("INSERT INTO temporary_outputs(path) VALUES (?1)",[&recorded]).map_err(|e|e.to_string())?;
 let target=temp.path().to_str().ok_or("Chemin non pris en charge.")?;
 let input=&r.inputs[0];
 if engine=="qpdf" {
   for input in &r.inputs {
    // --is-encrypted returns 0 for encrypted and 2 for unencrypted PDF.
    let mut check=Command::new(&bin);check.args(["--is-encrypted",input]);
    let result=execute(check,cancel,false);
    if result.is_ok(){return Err("PDF protégé : déverrouillez-le avant de le traiter.".into())}
    if cancel.load(Ordering::SeqCst){return Err("Annulé".into())}
    execute(cmd(&bin,&["--check",input]),cancel,true)?;
   }
   let mut c=Command::new(&bin);c.args(["--empty","--pages"]);
   if r.operation=="pdf.extract" {
    let count=execute(cmd(&bin,&["--show-npages",input]),cancel,false)?.trim().parse::<u32>().map_err(|_|"Nombre de pages illisible.")?;
    if !pages_valid(&r.pages,count){return Err("Sélection de pages invalide ou hors du document.".into())}c.args([input,&r.pages]);
   }else{for p in &r.inputs{c.args([p,"1-z"]);}}
   c.args(["--",target]);execute(c,cancel,true)?;
   execute(cmd(&bin,&["--check",target]),cancel,true)?;
 }else if engine=="ffmpeg" {
   let probe=engines::executable(data,engine,true)?;
   execute(cmd(&probe,&["-v","error","-protocol_whitelist","file,pipe","-show_entries","format=duration","-of","default=noprint_wrappers=1",input]),cancel,false)?;
   let mut c=Command::new(&bin);c.args(["-nostdin","-v","error","-y","-protocol_whitelist","file,pipe","-i",input]);
   if r.operation=="video.convert" {c.args(["-map","0:v:0","-map","0:a:0?","-c:v","libx264","-preset","medium","-crf",&r.quality.to_string(),"-pix_fmt","yuv420p","-vf","pad=ceil(iw/2)*2:ceil(ih/2)*2","-c:a","aac","-b:a","192k","-movflags","+faststart"]);}
   else {c.args(["-map",&format!("0:a:{}",r.track),"-vn"]);match r.format.as_str(){"mp3"=>{c.args(["-c:a","libmp3lame","-b:a","192k"]);},"wav"=>{c.args(["-c:a","pcm_s16le"]);},_=>{c.args(["-c:a","flac"]);}}}
   c.arg(target);execute(c,cancel,false)?;
   execute(cmd(&probe,&["-v","error","-show_entries","format=format_name",target]),cancel,false)?;
 }else {
   let config=tempfile::tempdir().map_err(|e|e.to_string())?;
   fs::write(config.path().join("policy.xml"),include_str!("../policy.xml")).map_err(|e|e.to_string())?;
   let mut identify=cmd(&bin,&["identify","-format","%n\n",input]);identify.env("MAGICK_CONFIGURE_PATH",config.path());
   let frames=execute(identify,cancel,false)?;
   if frames.lines().any(|n|n.trim()!="1"){return Err("Les images animées ne sont pas prises en charge en V1.".into())}
   let mut c=Command::new(&bin);c.env("MAGICK_CONFIGURE_PATH",config.path()).args([input,"-auto-orient"]);
   if r.operation=="image.resize"{c.args(["-resize",&format!("{}x>",r.width)]);}
   if r.format=="jpeg"{c.args(["-background","white","-alpha","remove","-alpha","off"]);}
   c.arg(target);execute(c,cancel,false)?;
 }
 if cancel.load(Ordering::SeqCst){return Err("Annulé".into())}
 if temp.as_file().metadata().map_err(|e|e.to_string())?.len()==0{return Err("Le résultat est vide.".into())}
 let base=if r.operation=="pdf.merge"{"fusion".to_string()}else{Path::new(input).file_stem().ok_or("Nom de fichier invalide.")?.to_string_lossy().into_owned()};
 let mut temp=temp;
 for suffix in 0..10000 {
 let name=if suffix==0{format!("{base}_toolkit.{}",r.format)}else{format!("{base}_toolkit_{suffix}.{}",r.format)};
 let out=dest.join(name);
 match temp.persist_noclobber(&out){Ok(_)=>{let _=crate::store::connect(db)?.execute("DELETE FROM temporary_outputs WHERE path=?1",[&recorded]);return Ok(out)},Err(e) if e.error.kind()==std::io::ErrorKind::AlreadyExists=>{temp=e.file;},Err(e)=>return Err(format!("Impossible de conserver le résultat : {}",e.error))}
 }Err("Trop de fichiers portent déjà ce nom.".into())
}
