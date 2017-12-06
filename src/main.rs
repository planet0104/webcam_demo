extern crate ws;
extern crate rscam;
extern crate uuid;
extern crate x264_sys;
use std::process::Command;

//static http server
extern crate iron;
extern crate mount;
extern crate staticfile;
use iron::prelude::*;
use iron::{ status, Iron };
use mount::Mount;
use staticfile::Static;
use std::path::Path;
use std::io::{ BufReader, Read };
use std::thread;

//webcam server
mod x264_encoder;
mod webcam;
use webcam::Webcam;
use ws::{Handler, Handshake, Message, Sender, CloseCode};
use std::rc::Rc;
use std::sync::Arc;
use std::cell::RefCell;
use uuid::Uuid;

fn main() {

    let webcam = Rc::new(RefCell::new(
                Webcam::new("/dev/video0".to_string(), 15, 352, 288)
            ));

    let websocket = ws::Builder::new().build(|out|{
        println!("创建连接...");
        WebSocketServer{
            sender: Arc::new(out),
            webcam: Rc::clone(&webcam),
            cb_key: Uuid::new_v4().hyphenated().to_string()
        }
    }).unwrap();

    let local_ip = match ip_address("enp0s3"){
        Some(ip) => ip,
        _ => "127.0.0.1".to_string()
    };

    //start static http server
    let http_server_addr = format!("{}:8081", local_ip);
    println!("listen http://{}", http_server_addr);
    thread::spawn(move || {
        let mut mount = Mount::new();
        mount.mount("/", Static::new(Path::new("html")));
        println!("监听:");
        Iron::new(mount).http(http_server_addr).unwrap();
        println!("Http Server结束.");
    });

    if let Err(error) = websocket.listen(format!("{}:8082", local_ip)) {
        println!("Websocket Server启动失败: {:?}", error);
    }
    println!("Server结束.");
}

struct WebSocketServer {
    sender: Arc<Sender>,
    webcam: Rc<RefCell<Webcam>>,
    cb_key:String
}

impl WebSocketServer{

    fn join_webcam(&mut self)->ws::Result<()>{
        self.webcam.borrow_mut().join_webcam(self.cb_key.clone(), self.sender.clone());
        Ok(())
    }

    fn quit_webcam(&mut self)->ws::Result<()>{
        self.webcam.borrow_mut().quit_webcam(&self.cb_key);
        Ok(())
    }
}

impl Handler for WebSocketServer {
    fn on_open(&mut self, _shake: Handshake) -> ws::Result<()> {
        println!("客户端加入 {}", self.cb_key);
        self.join_webcam()
    }

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
	    println!("on_message {:?}", msg);
        Ok(())
    }

    fn on_close(&mut self, _code: CloseCode, _reason: &str) {
        self.quit_webcam().unwrap_or_default();
    }
}

//获取IP地址
fn ip_address<'a>(device:&'a str) -> Option<String>{
    let result = execute(format!("{}{}{}", "LANG=C ifconfig ", device, " $NIC | awk '/inet addr:/{ print $2 }' | awk -F: '{print $2 }'").as_ref());
    let local_ips = result.lines();
    
    for ip in local_ips{
        if ip == "127.0.0.1"{
            continue;
        }
        return Some(String::from(ip));    
    }
    None
}

//执行shell
pub fn execute<'a>(cmd:&'a str) ->String{
    let output = Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .output()
                .expect("failed to execute process");
    //let result = output.stdout;//Vec<u8>
    String::from_utf8(output.stdout).unwrap_or_else(|err|{
        println!("{:?}", err);
        String::from("")
    })
}