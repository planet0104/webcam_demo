extern crate rscam;
extern crate ws;

use x264_encoder::X264Encoder;
use ws::{Message, Sender};
use rscam::{Camera};
use std::thread;
use std::collections::HashMap;
use std::sync::mpsc::{channel, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

pub struct Webcam{
    senders:Arc<Mutex<HashMap<String, Arc<Sender>>>>,
    device:String,
    fps:u32,
    width:i32,
    height:i32,
    preset:String,
    running: Arc<Mutex<bool>>
}

impl Drop for Webcam {
    fn drop(&mut self) {
        println!("Webcam Dropping!");
    }
}

impl Webcam{
    pub fn new(device:String, fps:u32, width:i32, height:i32)->Webcam{
        Webcam{
            senders: Arc::new(Mutex::new(HashMap::new())),
            fps: fps,
            width: width,
            height: height,
            device: device,
            preset: "veryfast".to_string(),
            running: Arc::new(Mutex::new(false)),
        }
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

    pub fn preset(&mut self, preset:String){
        self.preset = preset;
    }

    //加入视频
    pub fn join_webcam(&mut self, key:String, receiver:Arc<Sender>){
        let mut senders = self.senders.lock().unwrap();
        if senders.contains_key(&key){
            return;
        }

        let mut self_running = self.running.lock().unwrap();
        senders.insert(key.clone(), receiver);
        if *self_running{
            return;
        }

        *self_running = true;
	    println!("启动Webcam.");
        //开启拍照线程
        let (data_sender, data_receiver) = channel();
        let (msg_sender, msg_receiver) = channel();
        let fps = self.fps;
	    let (width, height) = (self.width, self.height);
        let preset = self.preset.clone();
        let device = self.device.clone();

        thread::spawn(move|| {
            let encoder = X264Encoder::new(fps, preset, "zerolatency".to_string(), width, height);
            if encoder.is_err(){
                println!("X264Encoder初始化失败.");
                let err = encoder.err();
                data_sender.send(Err(format!("{:?}", err.clone()))).unwrap();
                return Err(format!("{:?}", err));
            }

            let mut encoder = encoder.unwrap();

            let open_result = Camera::new(&device);
            if open_result.is_ok(){
                println!("相机已打开.");
                let mut camera = open_result.unwrap();
                let start_result =  camera.start(&rscam::Config {
                    interval: (1, 30),
                    resolution: (width as u32, height as u32),
                    format: b"YU12",
                    ..Default::default()
                });
                if start_result.is_err(){
                    println!("相机启动失败");
                    println!("Webcam拍照线程结束.{:?}", start_result.err());
                }else{
                    println!("相机已启动.");
                    //检查是否要求结束
                    loop{
                        let msg_recv = msg_receiver.try_recv();
                        if msg_recv.is_ok(){
                            println!("Webcam拍照线程结束.{}", msg_recv.unwrap());
                        }else if msg_recv.err().unwrap() == TryRecvError::Disconnected{
                            println!("Webcam拍照线程结束.{:?}", msg_recv.err());
                            break;
                        }
                        let start_time = Instant::now();
                        let frame_resut = camera.capture();
                        if frame_resut.is_err(){
                            println!("拍照失败");
                            println!("Webcam拍照线程结束.{:?}", frame_resut.err());
                            break;
                        }
                        let frame = frame_resut.unwrap();
                        
                        let encode_result = encoder.encode(Vec::from(&frame[..]));
                        
                        if encode_result.is_err(){
                            data_sender.send(Err(format!("{:?}", encode_result.err()))).unwrap_or_default();
                            break;
                        }

                        let d = start_time.elapsed();

                        //发送图像数据
                        if let Err(err) = data_sender.send(Ok(encode_result.unwrap())){
                            println!("Webcam拍照线程结束.{:?}", err);
                            break;
                        }

                        //计算耗费的时间，并延迟
                        let elapsed_ms = (start_time.elapsed().subsec_nanos()/1_000_000) as u64;
                        if elapsed_ms<1000/fps as u64{
                            thread::sleep(Duration::from_millis(1000/fps as u64-elapsed_ms));
                        }
                    }
                    //关闭Camera
                    let _ = camera.stop().unwrap_or_default();
                }
                Ok(())
            }else{
                println!("相机打开失败.");
                data_sender.send(Err(String::from("相机开启失败!"))).unwrap();
                Err(format!("{:?}", open_result.err()))
            }
        });

        //开启接收线程
        let senders_clone = self.senders.clone();
        let running_clone = self.running.clone();
        thread::spawn(move|| {
            loop{
                let recv = data_receiver.recv();
                if recv.is_err() {
                    println!("{:?}", recv.err());
                    break;
                }else{
                    let data_result = recv.unwrap();
                    if data_result.is_err(){
                        println!("{:?}", data_result.err());
                        break;
                    }
                    let senders = senders_clone.lock().unwrap();
                    if senders.len() == 0{
                        println!("webcam.senders为空.");
                        msg_sender.send("结束.").unwrap_or_default();
                        break;
                    }
                    let frame_data = data_result.unwrap();
                    for (_key, sender) in senders.iter(){
                        //给Sender发送数据
                        let _ = sender.send(Message::binary(frame_data.as_slice())).unwrap_or_default();
                    }
                }
            }
            *running_clone.lock().unwrap() = false;
            senders_clone.lock().unwrap().clear();
            println!("Webcam结束.");
        });
        println!("Webcam启动.");
    }

    //退出视频
    pub fn quit_webcam(&mut self, key:&String){
        let mut senders = self.senders.lock().unwrap();
        senders.remove(key);
    }
}
