use super::*;
use std::io::{Cursor,Seek,SeekFrom};
fn artifact(bytes:&[u8])->Artifact{Artifact{id:"test".into(),version:"1".into(),platform:platform(),url:"https://github.com/Hicha-m/tool-kit/releases/download/test/test.zip".into(),sha256:format!("{:x}",Sha256::digest(bytes)),size:bytes.len() as u64,unpacked_limit:1000,executable:"bin/tool".into(),probe:None,license:"MIT".into()}}
fn archive(entries:&[(&str,&[u8])])->Vec<u8>{
 let mut out=zip::ZipWriter::new(Cursor::new(Vec::new()));
 for (name,bytes) in entries {out.start_file(*name,zip::write::SimpleFileOptions::default()).unwrap();out.write_all(bytes).unwrap();}
 out.finish().unwrap().into_inner()
}
fn file(bytes:&[u8])->fs::File{let mut f=tempfile::tempfile().unwrap();f.write_all(bytes).unwrap();f.seek(SeekFrom::Start(0)).unwrap();f}
#[test] fn regression_verified_archive_extracts_expected_bytes(){
 let bytes=archive(&[("bin/tool",b"portable executable")]);let a=artifact(&bytes);let mut downloaded=tempfile::tempfile().unwrap();copy_verified(&mut Cursor::new(&bytes),&mut downloaded,&a).unwrap();
 let dir=tempfile::tempdir().unwrap();extract_archive(downloaded,dir.path(),&a).unwrap();assert_eq!(fs::read(dir.path().join("bin/tool")).unwrap(),b"portable executable");
}
#[test] fn regression_wrong_checksum_truncation_and_oversized_download_are_rejected(){
 let bytes=archive(&[("bin/tool",b"bytes")]);let a=artifact(&bytes);
 let mut wrong=a.clone();wrong.sha256="0".repeat(64);assert!(copy_verified(&mut Cursor::new(&bytes),&mut tempfile::tempfile().unwrap(),&wrong).unwrap_err().contains("Intégrité"));
 assert!(copy_verified(&mut Cursor::new(&bytes[..bytes.len()-1]),&mut tempfile::tempfile().unwrap(),&a).is_err());
 let mut smaller=a.clone();smaller.size=1;assert!(copy_verified(&mut Cursor::new(&bytes),&mut tempfile::tempfile().unwrap(),&smaller).is_err());
}
#[test] fn regression_interrupted_download_reports_retriable_failure(){
 struct Broken;impl Read for Broken{fn read(&mut self,_:&mut [u8])->std::io::Result<usize>{Err(std::io::Error::new(std::io::ErrorKind::ConnectionReset,"interrupted"))}}
 let a=artifact(b"payload");assert!(copy_verified(&mut Broken,&mut tempfile::tempfile().unwrap(),&a).unwrap_err().contains("interrompu"));
}
#[test] fn regression_extraction_rejects_traversal_absolute_and_nonportable_paths(){
 for name in ["../outside", "/outside", "C:/outside", "bin\\evil"]{
 let bytes=archive(&[(name,b"bad")]);let dir=tempfile::tempdir().unwrap();assert!(extract_archive(file(&bytes),dir.path(),&artifact(&bytes)).is_err(),"{name}");}
}
#[test] fn regression_extraction_enforces_size_budget_and_preserves_old_installation(){
 let dir=tempfile::tempdir().unwrap();let old=dir.path().join("1");fs::create_dir(&old).unwrap();fs::write(old.join("tool"),b"old version").unwrap();
 {let staging=tempfile::tempdir_in(dir.path()).unwrap();let bytes=archive(&[("bin/tool",b"too much data")]);let mut a=artifact(&bytes);a.unpacked_limit=1;assert!(extract_archive(file(&bytes),staging.path(),&a).is_err());}
 assert_eq!(fs::read(old.join("tool")).unwrap(),b"old version");assert_eq!(fs::read_dir(dir.path()).unwrap().count(),1);
}
#[test] fn regression_extraction_rejects_symlinks(){
 let mut w=zip::ZipWriter::new(Cursor::new(Vec::new()));w.add_symlink("bin/tool","../../outside",zip::write::SimpleFileOptions::default()).unwrap();let bytes=w.finish().unwrap().into_inner();
 let dir=tempfile::tempdir().unwrap();assert!(extract_archive(file(&bytes),dir.path(),&artifact(&bytes)).unwrap_err().contains("symboliques"));
}
#[test] fn regression_catalogue_remains_fail_closed(){
 let dir=tempfile::tempdir().unwrap();assert!(install(dir.path(),"untrusted-plugin").is_err());assert!(!dir.path().join("engines").exists());assert!(relative("../escape").is_err());assert!(relative("/absolute").is_err());
}
