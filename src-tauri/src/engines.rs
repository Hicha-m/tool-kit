use crate::model::{Artifact, Catalog, Engine};
use sha2::{Digest,Sha256};
use std::{fs,io::{Read,Write},path::{Path,PathBuf},process::{Command,Stdio}};

pub fn platform()->String {format!("{}-{}",std::env::consts::OS,std::env::consts::ARCH)}
pub fn catalog()->Result<Catalog,String> {
 let c:Catalog=serde_json::from_str(include_str!("../catalog/engines.json")).map_err(|e|e.to_string())?;
 if c.schema_version!=1{return Err("Version de catalogue incompatible.".into())}Ok(c)
}
pub fn artifact(id:&str)->Result<Artifact,String> {
 catalog()?.artifacts.into_iter().find(|a|a.id==id&&a.platform==platform()).ok_or_else(||"Aucune distribution qualifiée pour ce moteur sur cette plateforme.".into())
}
pub fn root(data:&Path,a:&Artifact)->PathBuf {data.join("engines").join(&a.id).join(&a.version)}
fn relative(s:&str)->Result<&Path,String> {
 let p=Path::new(s);
 if p.is_absolute()||p.components().any(|c|!matches!(c,std::path::Component::Normal(_))) {return Err("Chemin de moteur invalide.".into())}Ok(p)
}
pub fn executable(data:&Path,id:&str,probe:bool)->Result<PathBuf,String> {
 let a=artifact(id)?;
 let entry=if probe {a.probe.as_deref().ok_or("Outil d’inspection absent.")?}else{&a.executable};
 let dir=root(data,&a);
 if !dir.join(".verified").is_file(){return Err("Installez d’abord le moteur depuis les paramètres.".into())}
 let p=dir.join(relative(entry)?);
 if !p.is_file(){return Err("Installation incomplète : réinstallez le moteur.".into())}Ok(p)
}
pub fn list(data:&Path)->Result<Vec<Engine>,String> {
 Ok(["qpdf","ffmpeg","imagemagick"].iter().map(|id| {
 let a=artifact(id).ok();let installed=executable(data,id,false).is_ok();
 Engine{id:id.to_string(),installed,version:a.as_ref().filter(|_|installed).map(|a|a.version.clone()),available:a.is_some()}
 }).collect())
}
pub fn install(data:&Path,id:&str)->Result<(),String> {
 let a=artifact(id)?;
 if !a.url.starts_with("https://github.com/Hicha-m/tool-kit/releases/download/") || a.size==0 || a.sha256.len()!=64 || a.license.is_empty(){return Err("Entrée du catalogue non qualifiée.".into())}
 relative(&a.id)?;relative(&a.version)?;relative(&a.executable)?;
 let parent=data.join("engines").join(&a.id);fs::create_dir_all(&parent).map_err(|e|e.to_string())?;
 // Every mirrored bundle is normalized to ZIP during qualification.
 let staging=tempfile::Builder::new().prefix(".install-").tempdir_in(&parent).map_err(|e|e.to_string())?;
 let mut archive=tempfile::tempfile().map_err(|e|e.to_string())?;
 let client=reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(600)).build().map_err(|e|e.to_string())?;
 let mut response=client.get(&a.url).send().map_err(|_|"Téléchargement impossible. Vérifiez votre connexion.")?.error_for_status().map_err(|_|"Le téléchargement est indisponible.")?;
 copy_verified(&mut response,&mut archive,&a)?;
 extract_archive(archive,staging.path(),&a)?;
 let bin=staging.path().join(&a.executable);
 let arg=if id=="qpdf"{"--version"}else{"-version"};
 let result=Command::new(bin).arg(arg).stdout(Stdio::null()).stderr(Stdio::null()).status().map_err(|_|"Le moteur ne démarre pas sur cet appareil.")?;
 if !result.success(){return Err("Validation du moteur échouée.".into())}
 if let Some(probe)=&a.probe {if !staging.path().join(relative(probe)?).is_file(){return Err("Outil d’inspection absent.".into())}}
 fs::write(staging.path().join(".verified"),&a.sha256).map_err(|e|e.to_string())?;
 let target=root(data,&a);if target.exists(){return Err("Cette version est déjà installée. L’installation existante est conservée.".into())}
 fs::rename(staging.path(),target).map_err(|e|e.to_string())?;Ok(())
}

fn copy_verified(response:&mut impl Read,archive:&mut fs::File,a:&Artifact)->Result<(),String>{
 let mut hash=Sha256::new();let mut total=0u64;let mut buffer=[0u8;65536];
 loop {let n=response.read(&mut buffer).map_err(|_|"Téléchargement interrompu. Vous pouvez réessayer.")?;if n==0{break}total+=n as u64;if total>a.size{return Err("Archive plus grande que prévu.".into())}hash.update(&buffer[..n]);archive.write_all(&buffer[..n]).map_err(|e|e.to_string())?;}
 if total!=a.size || format!("{:x}",hash.finalize())!=a.sha256.to_lowercase(){return Err("Intégrité incorrecte : téléchargement refusé.".into())}
 Ok(())
}
fn extract_archive(mut archive:fs::File,staging:&Path,a:&Artifact)->Result<(),String>{
 use std::io::{Seek,SeekFrom};archive.seek(SeekFrom::Start(0)).map_err(|e|e.to_string())?;
 let mut zip=zip::ZipArchive::new(archive).map_err(|_|"Archive ZIP invalide.")?;let mut unpacked=0u64;
 for i in 0..zip.len(){let mut entry=zip.by_index(i).map_err(|e|e.to_string())?;
 if entry.name().contains('\\')||entry.name().split('/').any(|part|part.contains(':')){return Err("Chemin non portable dans l’archive.".into())}
 let name=entry.enclosed_name().ok_or("Chemin dangereux dans l’archive.")?.to_path_buf();
 if entry.unix_mode().map(|m|m&0o170000==0o120000).unwrap_or(false){return Err("Liens symboliques interdits dans l’archive.".into())}
 unpacked=unpacked.checked_add(entry.size()).ok_or("Archive trop volumineuse.")?;if unpacked>a.unpacked_limit{return Err("Archive trop volumineuse.".into())}
 let out=staging.join(name);
 if entry.is_dir(){fs::create_dir_all(out).map_err(|e|e.to_string())?;continue}
 fs::create_dir_all(out.parent().ok_or("Chemin invalide.")?).map_err(|e|e.to_string())?;
 let expected=entry.size();
 let mut file=fs::File::create_new(&out).map_err(|_|"Archive contenant des chemins dupliqués ou incompatibles.")?;
 let written=std::io::copy(&mut (&mut entry).take(expected.saturating_add(1)),&mut file).map_err(|e|e.to_string())?;
 if written!=expected{return Err("Taille de fichier extraite incorrecte.".into())}
 #[cfg(unix)] {use std::os::unix::fs::PermissionsExt;let mode=entry.unix_mode().unwrap_or(0o644)&0o777;fs::set_permissions(&out,fs::Permissions::from_mode(mode)).map_err(|e|e.to_string())?;}
 }
 Ok(())
}
#[cfg(test)]
#[path="tests/installer_regression.rs"]
mod regression;
