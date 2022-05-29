use ws::{Sender, listen};
use std::thread;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use anyhow::Result;
use crate::camera;

pub struct Webcam{
    device:String,
    fps:u32,
    width:i32,
    height:i32,
    preview_width: i32,
    preview_height: i32,
}

/// 启动websocket服务器
pub fn start(ip: &str, port: &str, device:&str, format:&str, fps:u32, width:i32, height:i32, preview_width: i32, preview_height: i32) -> Result<()>{
    
    //初始化
    dcv_color_primitives::initialize();

    let webcam = Arc::new(Mutex::new(Webcam::new(device, format, fps, width, height, preview_width, preview_height)?));

    let addr = format!("{}:{}", ip, port);
    println!("启动websocket服务器:{addr}");

    listen(&addr, |out| {
        
        if let Ok(mut webcam) = webcam.lock(){
            let key = uuid::Uuid::new_v4().to_string();
            let _ = webcam.join_webcam(key, out);
        }
        move |_msg|{
            Ok(())
        }
    })?;
    println!("websocket服务器结束");
    Ok(())
}

impl Webcam{
    pub fn new(device:&str, format:&str, fps:u32, width:i32, height:i32, preview_width: i32, preview_height: i32)-> Result<Webcam>{
        camera::open_camera(device, format, width, height, preview_width, preview_height, fps, None)?;

        Ok(Webcam{
            device: device.to_string(),
            fps,
            width,
            height,
            preview_width,
            preview_height,
        })
    }

    pub fn fps(&mut self, fps:u32){
	    self.fps = fps;
    }

    pub fn width(&mut self, width:i32){
	    self.width = width;
    }

    pub fn height(&mut self, height:i32){
	    self.height = height;
    }

    /// 加入视频
    pub fn join_webcam(&mut self, key:String, sender:Sender) -> Result<()>{
        
        let (frame_sender, frame_receiver) = channel();

        // 放入拍照线程中
        camera::add_receiver(key.clone(), frame_sender)?;

        // 启动websocket循环发送线程
        thread::spawn(move || -> Result<()>{
            loop{
                if let Ok(frame_data) = frame_receiver.try_recv(){
                    // 发送给客户端, 报错以后结束线程
                    if let Err(err) = sender.send(frame_data){
                        eprintln!("{:?}", err);
                        break;
                    }
                }
                thread::sleep(Duration::from_millis(10));
            }
            let res = camera::remove_receiver(&key);
            println!("客户端退出: {:?}", res);
            Ok(())
        });

        Ok(())
    }

}
