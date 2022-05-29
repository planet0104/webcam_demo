
//https://github.com/matijagaspar/ws-avc-player

extern crate webcam;

fn main() {
    webcam::start("127.0.0.1", "8080",
        "/dev/video0", "YU12", 10,
        4000, 3000,
        1600, 1200).unwrap();
    
        // 启动以后打开 html中的index.html开始预览
}