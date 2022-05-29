use core::slice;
use std::{os::raw::c_int, ffi::CStr};

mod camera;
mod webcam;
mod x264_encoder;

pub use webcam::*;
pub use camera::close_camera;
pub use camera::get_current_frame;
pub use camera::get_current_preview;


// #[no_mangle]
// extern "C" fn get_current_preview(data_ptr: *mut u8, len: c_int) -> c_int{
//     match camera::get_current_preview(){
//         Ok(data) => {
//             match data{
//                 Some(data) => {
//                     if len != data.len() as i32{
//                         eprintln!("get_current_preview 长度不一致 data.len={}", data.len());
//                         -1
//                     }else{
//                         let dst_data = unsafe{ slice::from_raw_parts_mut(data_ptr, len as usize) };
//                         dst_data.copy_from_slice(&data);
//                         0
//                     }
//                 }
//                 None => {
//                     eprintln!("get_current_preview 没有帧数据");
//                     -1
//                 }
//             }
//         }
//         Err(err) => {
//             eprintln!("get_current_preview 当前帧读取失败: {:?}", err);
//             -1
//         }
//     }
// }


// #[no_mangle]
// extern "C" fn get_current_frame(data_ptr: *mut u8, len: c_int) -> c_int{
//     match camera::get_current_frame(){
//         Ok(data) => {
//             match data{
//                 Some(data) => {
//                     if len != data.len() as i32{
//                         eprintln!("get_current_frame 长度不一致 data.len={}", data.len());
//                         -1
//                     }else{
//                         let dst_data = unsafe{ slice::from_raw_parts_mut(data_ptr, len as usize) };
//                         dst_data.copy_from_slice(&data);
//                         0
//                     }
//                 }
//                 None => {
//                     eprintln!("get_current_frame 没有帧数据");
//                     -1
//                 }
//             }
//         }
//         Err(err) => {
//             eprintln!("get_current_frame 当前帧读取失败: {:?}", err);
//             -1
//         }
//     }
// }

// #[no_mangle]
// extern "C" fn close_camera(){
//     let res = camera::close_camera();
//     println!("close_camera: {:?}", res);
// }

// #[no_mangle]
// extern "C" fn start(ip: *const i8, port: *const i8, device: *const i8, format: *const i8, fps:c_int, width:c_int, height:c_int, preview_width: c_int, preview_height: c_int) -> c_int {
//     unsafe{
//         let ip = CStr::from_ptr(ip).to_str();
//         let port = CStr::from_ptr(port).to_str();
//         let device = CStr::from_ptr(device).to_str();
//         let format = CStr::from_ptr(format).to_str();

//         if ip.is_err(){
//             eprintln!("{:?}", ip.err());
//             return -1;
//         }
//         if port.is_err(){
//             eprintln!("{:?}", port.err());
//             return -1;
//         }
//         if device.is_err(){
//             eprintln!("{:?}", device.err());
//             return -1;
//         }
//         if format.is_err(){
//             eprintln!("{:?}", format.err());
//             return -1;
//         }
//         let ip = ip.unwrap();
//         let port = port.unwrap();
//         let device = device.unwrap();
//         let format = format.unwrap();

//         match webcam::start(ip, port, device, format, fps as u32, width, height, preview_width, preview_height){
//             Ok(()) => 0,
//             Err(err) => {
//                 eprintln!("webcam 运行失败: {:?}", err);
//                 -1
//             }
//         }
//     }
// }