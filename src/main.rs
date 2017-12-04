extern crate ws;
extern crate rscam;
extern crate uuid;
extern crate x264_sys;

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
                Webcam::new("/dev/video0".to_string(), 16, 352, 288)
            ));

    let websocket = ws::Builder::new().build(|out|{
        println!("创建连接...");
        WebSocketServer{
            sender: Arc::new(out),
            webcam: Rc::clone(&webcam),
            cb_key: Uuid::new_v4().hyphenated().to_string()
        }
    }).unwrap();

    println!("Server启动... ws::127.0.0.1:8080");
    if let Err(error) = websocket.listen("192.168.201.2:8080") {
        println!("Server启动失败: {:?}", error);
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
