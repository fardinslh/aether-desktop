use std::{io::{Read,Write}, os::unix::net::UnixStream, time::Duration};
use serde_json::{json,Value};
pub const SOCKET:&str="/var/run/com.aether.desktop.router.sock";
pub fn request(command:Value)->Result<Value,String> {
    let mut socket=UnixStream::connect(SOCKET).map_err(|e|format!("Install Aether Desktop.pkg to enable the system VPN helper: {}",e))?;
    socket.set_read_timeout(Some(Duration::from_secs(10))).map_err(|e|e.to_string())?;
    socket.set_write_timeout(Some(Duration::from_secs(3))).map_err(|e|e.to_string())?;
    socket.write_all(&serde_json::to_vec(&command).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    socket.shutdown(std::net::Shutdown::Write).map_err(|e|e.to_string())?;
    let mut reply=String::new();socket.take(65536).read_to_string(&mut reply).map_err(|e|e.to_string())?;
    let value:Value=serde_json::from_str(&reply).map_err(|e|e.to_string())?;
    if let Some(e)=value.get("error").and_then(Value::as_str) {return Err(e.into());} Ok(value)
}
pub fn status()->Result<Value,String> {request(json!({"command":"status"}))}
pub fn stop(){let _=request(json!({"command":"stop"}));}
