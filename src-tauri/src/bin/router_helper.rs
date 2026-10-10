#[cfg(not(target_os="macos"))] fn main() {}
#[cfg(target_os="macos")] fn main() { if let Err(e)=service::run() {eprintln!("{}",e);std::process::exit(1);} }
#[cfg(target_os="macos")]
mod service {
use std::{fs, io::{Read,Write}, os::unix::{fs::PermissionsExt,net::{UnixListener,UnixStream},io::AsRawFd,process::CommandExt}, path::Path,process::{Child,Command,Stdio},time::{Duration,Instant}};
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
const BASE:&str="/Library/Application Support/AetherDesktop/router";
const SOCKET:&str="/var/run/com.aether.desktop.router.sock";
const CLIENT:&str="/Applications/Aether Desktop.app/Contents/MacOS/aether-desktop";
#[link(name="proc")] unsafe extern "C" {fn proc_pidpath(pid:i32,buffer:*mut libc::c_void,size:u32)->i32;}
fn hash(path:&Path)->Result<String,String>{Ok(format!("{:x}",Sha256::digest(fs::read(path).map_err(|e|e.to_string())?)))}
fn authenticate(stream:&UnixStream,manifest:&Value)->Result<(),String>{
 let mut uid=0;let mut gid=0;let mut pid:i32=0;let mut size=std::mem::size_of_val(&pid) as libc::socklen_t;
 unsafe { if libc::getpeereid(stream.as_raw_fd(),&mut uid,&mut gid)!=0 || libc::getsockopt(stream.as_raw_fd(),0,2,&mut pid as *mut _ as *mut _,&mut size)!=0 {return Err("Peer credentials unavailable".into());} }
 if Some(uid as u64)!=manifest["uid"].as_u64(){return Err("Unauthorized user".into());}
 let mut path=[0u8;4096];let n=unsafe{proc_pidpath(pid,path.as_mut_ptr() as *mut _,path.len() as u32)};
 if n<=0 {return Err("Peer executable unavailable".into());}
 let actual=std::ffi::CStr::from_bytes_until_nul(&path).map_err(|e|e.to_string())?.to_str().map_err(|e|e.to_string())?;
 if actual!=CLIENT || Some(hash(Path::new(CLIENT))?.as_str())!=manifest["clientSha256"].as_str(){return Err("Unverified client".into());} Ok(())
}
/// Reject all caller-controlled filesystem writes, loaders, external APIs and experimental features.
fn validate_config(v:&Value)->Result<(),String>{
 fn walk(v:&Value)->Result<(),String>{match v {
  Value::Object(o)=>for(k,v) in o {if ["path","output","certificate_path","key_path","experimental","cache_file","clash_api","v2ray_api","include","directory","script"].contains(&k.as_str()){return Err(format!("Forbidden config field {}",k));}walk(v)?;},
  Value::Array(a)=>for v in a {walk(v)?;},_=>{}
 }Ok(())}
 walk(v)?;
 let inbounds=v["inbounds"].as_array().ok_or("Missing TUN")?;
 if inbounds.len()!=1 || inbounds[0]["type"]!="tun" || inbounds[0]["interface_name"]!="utun99" {return Err("Only the managed utun99 inbound is permitted".into());}
 if v["log"]["output"].is_string(){return Err("File logging forbidden".into());} Ok(())
}
// A root-owned PID record lets launchd recover the router after a helper crash.
fn clean_previous_router(router: &Path) {
 let record=Path::new(BASE).join("router.pid");
 if let Some(pid)=fs::read_to_string(&record).ok().and_then(|s|s.trim().parse::<i32>().ok()).filter(|p|*p>1) {
  let mut path=[0u8;4096];
  let n=unsafe{proc_pidpath(pid,path.as_mut_ptr() as *mut _,path.len() as u32)};
  let actual=std::ffi::CStr::from_bytes_until_nul(&path).ok().and_then(|s|s.to_str().ok());
  if n>0 && actual==router.to_str() && unsafe{libc::getpgid(pid)}==pid {
   unsafe{libc::kill(-pid,libc::SIGTERM);}
   let deadline=Instant::now()+Duration::from_secs(2);
   while Instant::now()<deadline && unsafe{libc::kill(pid,0)}==0 {std::thread::sleep(Duration::from_millis(50));}
   if unsafe{libc::kill(pid,0)}==0 {unsafe{libc::kill(-pid,libc::SIGKILL);}}
  }
 }
 let _=fs::remove_file(record);
}
fn stop(child:&mut Option<Child>){let _=fs::remove_file(Path::new(BASE).join("router.pid"));if let Some(mut c)=child.take(){unsafe{libc::kill(-(c.id() as i32),libc::SIGTERM);}let deadline=Instant::now()+Duration::from_secs(2);while Instant::now()<deadline {if c.try_wait().ok().flatten().is_some(){return;}std::thread::sleep(Duration::from_millis(50));}unsafe{libc::kill(-(c.id() as i32),libc::SIGKILL);}let _=c.wait();}}
pub fn run()->Result<(),String>{
 if unsafe{libc::geteuid()}!=0{return Err("Helper requires launchd root context".into());}
 let manifest:Value=serde_json::from_slice(&fs::read(format!("{}/installed.json",BASE)).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
 let router=Path::new(BASE).join("sing-box");
 if Some(hash(&router)?.as_str())!=manifest["routerSha256"].as_str(){return Err("Router checksum mismatch".into());}
 clean_previous_router(&router);
 let _=fs::remove_file(SOCKET);let listener=UnixListener::bind(SOCKET).map_err(|e|e.to_string())?;
 fs::set_permissions(SOCKET,fs::Permissions::from_mode(0o666)).map_err(|e|e.to_string())?;
 listener.set_nonblocking(true).map_err(|e|e.to_string())?;
 let mut child:Option<Child>=None;let mut last=Instant::now();
 loop {
  if last.elapsed()>Duration::from_secs(20){stop(&mut child);}
  match listener.accept(){
   Ok((mut socket,_))=>{
    let result=(||->Result<Value,String>{
     authenticate(&socket,&manifest)?;
     socket.set_read_timeout(Some(Duration::from_secs(2))).map_err(|e|e.to_string())?;
     let mut data=Vec::new();(&mut socket).take(2_000_001).read_to_end(&mut data).map_err(|e|e.to_string())?;
     if data.len()>2_000_000{return Err("Request too large".into());}
     let request:Value=serde_json::from_slice(&data).map_err(|e|e.to_string())?;
     match request["command"].as_str().unwrap_or(""){
      "status"=>{},"stop"=>stop(&mut child),
      "start"|"apply"=>{
       if request["command"]=="start" && child.as_mut().is_some_and(|c|c.try_wait().ok().flatten().is_none()){return Err("Router already active".into());}
       let config=&request["config"];validate_config(config)?;
       if Some(hash(&router)?.as_str())!=manifest["routerSha256"].as_str(){return Err("Router checksum mismatch".into());}
       let path=Path::new(BASE).join("candidate.json");fs::write(&path,serde_json::to_vec(config).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
       let checked=Command::new(&router).args(["check","-c"]).arg(&path).stdin(Stdio::null()).output().map_err(|e|e.to_string())?;
       if !checked.status.success(){return Err(String::from_utf8_lossy(&checked.stderr).into());}
       stop(&mut child);
       fs::rename(&path,Path::new(BASE).join("active.json")).map_err(|e|e.to_string())?;
       let log=fs::OpenOptions::new().create(true).append(true).open(Path::new(BASE).join("router.log")).map_err(|e|e.to_string())?;
       child=Some(Command::new(&router).args(["run","-c"]).arg(Path::new(BASE).join("active.json")).current_dir(BASE).stdin(Stdio::null()).stdout(log.try_clone().map_err(|e|e.to_string())?).stderr(log).process_group(0).spawn().map_err(|e|e.to_string())?);
       if let Some(c)=child.as_ref(){if let Err(e)=fs::write(Path::new(BASE).join("router.pid"),c.id().to_string()){stop(&mut child);return Err(e.to_string());}}
      },_=>return Err("Unknown command".into())
     }
     last=Instant::now();let pid=child.as_mut().and_then(|c|if c.try_wait().ok().flatten().is_none(){Some(c.id())}else{None});
     Ok(json!({"pid":pid,"running":pid.is_some()}))
    })();
    let reply=result.unwrap_or_else(|e|json!({"error":e}));let _=socket.write_all(reply.to_string().as_bytes());
   },Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>std::thread::sleep(Duration::from_millis(100)),Err(e)=>return Err(e.to_string())
  }
 }
}
#[cfg(test)]mod tests{use super::*;#[test]fn no_privileged_file_paths(){assert!(validate_config(&json!({"inbounds":[{"type":"tun","interface_name":"utun99"}],"log":{"output":"/etc/passwd"}})).is_err());}}
}
