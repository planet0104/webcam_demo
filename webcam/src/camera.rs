use std::{thread, sync::{Mutex, mpsc::Sender, RwLock}, time::{Instant, Duration}, num::NonZeroU32, collections::HashMap};

use anyhow::{anyhow, Result};
use dcv_color_primitives::{convert_image, PixelFormat, ImageFormat, ColorSpace};
use fast_image_resize::{Resizer, ResizeAlg, FilterType, ImageView, ImageViewMut, PixelType};
use once_cell::sync::Lazy;
use rscam::Camera;
use crate::x264_encoder::X264Encoder;

//sudo apt install libx264-dev

// 相机是否打开
static IS_RUNNING: Lazy<Mutex<bool>> = Lazy::new(|| {
    Mutex::new(false)
});

// 接收者
static RECEIVERS: Lazy<Mutex<HashMap<String, Sender<Vec<u8>>>>> = Lazy::new(||{
    Mutex::new(HashMap::new())
});

pub fn add_receiver(name: String, sender: Sender<Vec<u8>>) -> Result<()>{
    let mut receivers = RECEIVERS.lock().map_err(|err| anyhow!("{:?}", err))?;
    receivers.insert(name, sender);
    Ok(())
}

pub fn remove_receiver(name: &str) -> Result<()>{
    let mut receivers = RECEIVERS.lock().map_err(|err| anyhow!("{:?}", err))?;
    receivers.remove(name);
    Ok(())
}

// 当前照片大图
static FRAME_BGRA: Lazy<RwLock<Option<Vec<u8>>>> = Lazy::new(||{
    RwLock::new(None)
});

fn set_frame_rgba(data:Vec<u8>) -> Result<()>{
    let mut frame = FRAME_BGRA.write().map_err(|err| anyhow!("{:?}", err))?;
    frame.replace(data);
    Ok(())
}

// 当前照片预览图
static PREVIEW_BGRA: Lazy<RwLock<Option<Vec<u8>>>> = Lazy::new(||{
    RwLock::new(None)
});

fn set_preview_rgba(data:Vec<u8>) -> Result<()>{
    let mut preview = PREVIEW_BGRA.write().map_err(|err| anyhow!("{:?}", err))?;
    preview.replace(data);
    Ok(())
}

/// BGRA
pub fn get_current_frame() -> Result<Option<Vec<u8>>>{
    let frame = FRAME_BGRA.read().map_err(|err| anyhow!("{:?}", err))?;
    Ok(frame.clone())
}

/// BGRA
pub fn get_current_preview() -> Result<Option<Vec<u8>>>{
    let preview = PREVIEW_BGRA.read().map_err(|err| anyhow!("{:?}", err))?;
    Ok(preview.clone())
}

pub fn is_running() -> Result<bool>{
    let is_running = IS_RUNNING.lock().map_err(|err| anyhow!("{:?}", err))?;
    Ok(*is_running)
}

fn set_running(run: bool) -> Result<()>{
    let mut is_running = IS_RUNNING.lock().map_err(|err| anyhow!("{:?}", err))?;
    *is_running = run;
    Ok(())
}

/// 关闭相机
pub fn close_camera() -> Result<()>{
    let mut is_running = IS_RUNNING.lock().map_err(|err| anyhow!("{:?}", err))?;
    *is_running = false;
    Ok(())
}

/// 打开相机
pub fn open_camera(device:&str, format:&str, width: i32, height: i32,
    preview_width: i32,
    preview_height: i32,
    fps: u32, preset: Option<String>) -> Result<()>{

    if width == 0 || height == 0 || preview_width == 0 || preview_height == 0{
        return Err(anyhow!("宽和高不能为0"));
    }

    if format != "YU12"{
        return Err(anyhow!("只支持YU12"));
    }

    if is_running()? {
        return Err(anyhow!("相机已经启动，请先结束"));
    }

    let mut camera = rscam::new(device)?;

    camera.start(&rscam::Config {
        interval: (1, fps),
        resolution: (width as u32, height as u32),
        format: format.as_bytes(), // YU12 (I420)
        ..Default::default()
    })?;

    let tune = "zerolatency".to_string();
    let preset = preset.unwrap_or("veryfast".to_string());

    thread::spawn(move|| -> Result<()> {
        set_running(true)?;

        // 相机原始大小的Enoder，用于计算，实际上不用于编码
        let encoder_big = X264Encoder::new(fps, preset.clone(), tune.clone(), width, height)
            .map_err(|err| anyhow!("{:?}", err))?;
        
        // 预览大小的Encoder，用于转码x264
        let encoder_small = X264Encoder::new(fps, preset, tune, preview_width, preview_height)
        .map_err(|err| anyhow!("{:?}", err))?;

        let err = run_capture_loop(camera, fps, encoder_big, encoder_small);
        if err.is_err(){
            eprintln!("相机错误:{:?}", err);
        }
        set_running(false)?;
        
        Ok(())
    });

    Ok(())
}

fn run_capture_loop(mut camera: Camera, fps:u32, encoder_big: X264Encoder, mut encoder_small:X264Encoder) -> Result<()>{
    println!("相机已启动.");
    loop{
        //检查是否要求结束
        if !is_running()?{
            break;
        }
        let start_time = Instant::now();

        //拍照
        let big_frame = camera.capture()?;

        //https://paaatrick.com/2020-01-26-yuv-pixel-formats/ 详解YUV
        
        //大图像数据YU12转BGRA，便于缩放
        let big_yu12_frame = Vec::from(&big_frame[..]);

        let plan_slice_0 = big_yu12_frame.get(..encoder_big.plane_size[0]).ok_or("plan_slice_0 长度错误").map_err(|err| anyhow!("{:?}", err))?;
        let plan_slice_1 = big_yu12_frame.get(encoder_big.plane_size[0]..encoder_big.plane_size[0]+encoder_big.plane_size[1]).ok_or("plan_slice_1 长度错误").map_err(|err| anyhow!("{:?}", err))?;
        let plan_slice_2 = big_yu12_frame.get(encoder_big.plane_size[0]+encoder_big.plane_size[1]..).ok_or("plan_slice_2 长度错误").map_err(|err| anyhow!("{:?}", err))?;

        let src_format = ImageFormat {
            pixel_format: PixelFormat::I420,
            color_space: ColorSpace::Bt601,
            num_planes: 3,
        };
    
        let dst_format = ImageFormat {
            pixel_format: PixelFormat::Bgra,
            color_space: ColorSpace::Rgb,
            num_planes: 1,
        };
        
        let mut dst_buffers = vec![0u8; encoder_big.width() as usize*encoder_big.height() as usize*4];

        convert_image(
            encoder_big.width() as u32,
            encoder_big.height() as u32,
            &src_format,
            None,
            &[&plan_slice_0, &plan_slice_1, &plan_slice_2],
            &dst_format,
            None,
            &mut[&mut dst_buffers],
        )?;

        //压缩
        // let mut resizer = Resizer::new(ResizeAlg::Nearest);
        let mut resizer = Resizer::new(ResizeAlg::Convolution(FilterType::Lanczos3));
        let src_view = ImageView::from_buffer(NonZeroU32::new(4000).unwrap(), NonZeroU32::new(3000).unwrap(), &dst_buffers, PixelType::U8x4)?;
        let mut dst_small = vec![0; encoder_small.width() as usize * encoder_small.height() as usize * 4];
        let mut dst_view = ImageViewMut::from_buffer(NonZeroU32::new(encoder_small.width() as u32).unwrap(), NonZeroU32::new(encoder_small.height() as u32).unwrap(), &mut dst_small, PixelType::U8x4)?;
        resizer.resize(&src_view, &mut dst_view)?;

        //存储大图
        set_frame_rgba(big_yu12_frame)?;

        //压缩后的BGRA 转 YU12

        let mut small_plan_slice_0 = vec![0u8; encoder_small.plane_size[0]];
        let mut small_plan_slice_1 = vec![0u8; encoder_small.plane_size[1]];
        let mut small_plan_slice_2 = vec![0u8; encoder_small.plane_size[2]];

        convert_image(
            encoder_small.width() as u32,
            encoder_small.height() as u32,
            &dst_format,
            None,
            &[& dst_small],
            &src_format,
            None,
            &mut [&mut small_plan_slice_0, &mut small_plan_slice_1, &mut small_plan_slice_2],
        )?;

        // 保存预览数据
        set_preview_rgba(dst_small)?;
        
        // 组装YU12数据
        let mut small_dst_buffer = Vec::with_capacity(encoder_small.plane_size[0]+encoder_small.plane_size[1]+encoder_small.plane_size[2]);
        small_dst_buffer.extend_from_slice(&small_plan_slice_0);
        small_dst_buffer.extend_from_slice(&small_plan_slice_1);
        small_dst_buffer.extend_from_slice(&small_plan_slice_2);

        let encode_result = encoder_small.encode(small_dst_buffer)
            .map_err(|err| anyhow!("{:?}", err))?;
 
        //发送图像数据
        let receivers = RECEIVERS.lock().map_err(|err| anyhow!("{:?}", err))?;
        for (_name, sender) in receivers.iter(){
            if let Err(err) = sender.send(encode_result.clone()){
                eprintln!("发送出错:{:?}", err);
            }
        }

        //计算耗费的时间，并延迟
        let elapsed_ms = (start_time.elapsed().subsec_nanos()/1_000_000) as u64;

        if elapsed_ms<1000/fps as u64{
            thread::sleep(Duration::from_millis(1000/fps as u64-elapsed_ms));
        }
    }

    camera.stop()?;

    println!("相机已结束.");

    Ok(())
}

//bgra转图片
// let mut rgba_data = Vec::with_capacity(dst_small.len());
// for pixel in dst_small.chunks_mut(4){
//     rgba_data.push(pixel[2]);
//     rgba_data.push(pixel[1]);
//     rgba_data.push(pixel[0]);
//     rgba_data.push(pixel[3]);
// }
// let image = RgbaImage::from_raw(small_width as u32, small_height as u32, rgba_data).unwrap();
// image.save("small.png").unwrap();

// panic!("两个都已经保存");

//bgra转图片
// let mut rgba_data = Vec::with_capacity(dst_buffers.len());
// for pixel in dst_buffers.chunks_mut(4){
//     rgba_data.push(pixel[2]);
//     rgba_data.push(pixel[1]);
//     rgba_data.push(pixel[0]);
//     rgba_data.push(pixel[3]);
// }
// let image = RgbaImage::from_raw(4000 as u32, 3000 as u32, rgba_data).unwrap();
// image.save("big.png").unwrap();


//`PixelFormat::I420` | `ColorSpace::Bt601(FR)`, `ColorSpace::Bt709(FR)`

// | Source pixel format  | Destination pixel formats  |
// | -------------------- | -------------------------- |
// | ARGB                 | I420, I444, NV12           |
// | BGR                  | I420, I444, NV12, RGB      |
// | BGRA                 | I420, I444, NV12, RGB      |
// | I420                 | BGRA                       |
// | I444                 | BGRA                       |
// | NV12                 | BGRA                       |
// | RGB                  | BGRA                       |

// pixel format        | color space
// --------------------|--------------------------------------
// `PixelFormat::Argb` | `ColorSpace::Rgb`
// `PixelFormat::Bgra` | `ColorSpace::Rgb`
// `PixelFormat::Bgr`  | `ColorSpace::Rgb`
// `PixelFormat::Rgba` | `ColorSpace::Rgb`
// `PixelFormat::Rgb`  | `ColorSpace::Rgb`
// `PixelFormat::I444` | `ColorSpace::Bt601(FR)`, `ColorSpace::Bt709(FR)`
// `PixelFormat::I422` | `ColorSpace::Bt601(FR)`, `ColorSpace::Bt709(FR)`
// `PixelFormat::I420` | `ColorSpace::Bt601(FR)`, `ColorSpace::Bt709(FR)`
// `PixelFormat::Nv12` | `ColorSpace::Bt601(FR)`, `ColorSpace::Bt709(FR)`

// pixel format        | subsampling | w   | h   | #planes | #1     | #2     | #3
// --------------------|:-----------:|:---:|:---:|:-------:|:------:|:------:|:-------:
// `PixelFormat::Argb` | 4:4:4       |     |     | 1       | argb:4 |        |
// `PixelFormat::Bgra` | 4:4:4       |     |     | 1       | bgra:4 |        |
// `PixelFormat::Bgr`  | 4:4:4       |     |     | 1       | bgr:3  |        |
// `PixelFormat::Rgba` | 4:4:4       |     |     | 1       | rgba:4 |        |
// `PixelFormat::Rgb`  | 4:4:4       |     |     | 1       | rgb:3  |        |
// `PixelFormat::I444` | 4:4:4       |     |     | 3       | y:1    | u:1    | v:1
// `PixelFormat::I422` | 4:2:2       |  2  |     | 1, 3    | y:1    | u:1/2  | v:1/2
// `PixelFormat::I420` | 4:2:0       |  2  |  2  | 3       | y:1    | u:1/4  | v:1/4
// `PixelFormat::Nv12` | 4:2:0       |  2  |  2  | 1, 2    | y:1    | uv:1/2 |