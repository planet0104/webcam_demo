//编译依赖
//sudo apt-get install libx264-dev 5M
//sudo apt-get install libclang-dev 217M
//sudo apt-get install clang 208M
//264转mp4
//ffmpeg -i output.264 output.mp4

//sudo apt install nasm

//参考:
//http://m.blog.csdn.net/liushu1231/article/details/9203239
//http://www.voidcn.com/article/p-qbdtxdiz-bew.html
use std::mem;
use std::slice;
use x264_dev::sys::X264NalT;
use x264_dev::sys::X264ParamT;
use x264_dev::sys::X264PictureT;
use x264_dev::sys::X264T;
use x264_dev::sys::x264_param_default_preset;
// use x264_sys::x264::*;
use x264_dev::sys::{x264_param_apply_profile, x264_encoder_encode, x264_encoder_open, x264_encoder_close, x264_picture_clean, x264_picture_init, x264_picture_alloc};
use x264_dev::raw::{X264_CSP_I444, X264_CSP_YV24, X264_CSP_BGR, X264_CSP_RGB, X264_CSP_BGRA, X264_CSP_YV12,X264_CSP_NV12,X264_CSP_NV21,X264_CSP_I422,X264_CSP_YV16,X264_CSP_NV16,X264_SYNC_LOOKAHEAD_AUTO, X264_CSP_MASK, X264_CSP_I420, X264_B_ADAPT_TRELLIS, X264_CSP_HIGH_DEPTH};
use std::os::raw::c_int;
use std::ffi::CString;

pub struct X264Encoder{
    pub plane_size: [usize; 3],
    nb_nal: c_int,
    c_nals: *mut X264NalT,
    pic_in: X264PictureT,
    pic_out: X264PictureT,
    current_frame: u32,
    enc: *mut X264T,
    width: i32,
    height: i32,
}

impl X264Encoder{
    pub fn new(frame_rate:u32, preset:String, tune:String, width: c_int, height: c_int)->Result<X264Encoder, &'static str>{
        //ultrafast, superfast, veryfast, fast, slow, veryslow
        // "zerolatency" 
        //let mut param = Param::default_preset("veryfast", "zerolatency").unwrap();
        let mut param:X264ParamT = unsafe { mem::zeroed() };
        match unsafe { x264_param_default_preset(&mut param as *mut X264ParamT,
                                                CString::new(preset.as_str()).unwrap().as_ptr(),
                                                CString::new(tune.as_str()).unwrap().as_ptr()) } {
            -1 => return Err("Invalid Argument"),
            0 => (),
            _ => return Err("Unexpected"),
        }

        param.i_width = width;
        param.i_height = height;

        //* cpuFlags
        param.i_threads  = X264_SYNC_LOOKAHEAD_AUTO;//* 取空缓冲区继续使用不死锁的保证.
        param.i_frame_total = 0;//* 编码总帧数.不知道用0.
        //* 流参数
        param.i_bframe = 5;
        param.b_open_gop = 0;
        //param.b_mb_tree = 0;//这个不为0,将导致编码延时帧...在实时编码时,必须为0
        param.i_bframe_pyramid = 0;
        param.i_bframe_adaptive = X264_B_ADAPT_TRELLIS as i32;
        //* Log参数，不需要打印编码信息时直接注释掉就行
        //param.i_log_level = X264_LOG_DEBUG as i32;

        //* 速率控制参数
        //param.rc.i_bitrate = 1024*10;//* 码率(比特率,单位Kbps)
        //* muxing parameters
        param.i_fps_den = 1; //* 帧率分母
        param.i_fps_num = frame_rate;//* 帧率分子
        //param.i_timebase_den = 1;
        //param.i_timebase_num = frame_rate;
        param.i_keyint_max = (frame_rate*2) as i32;

        //使用实时视频传输时，需要实时发送sps,pps数据
        //param.b_repeat_headers = 1;  // 重复SPS/PPS 放到关键帧前面

        //* 设置Profile.使用Baseline profile
        match unsafe { x264_param_apply_profile(&mut param, CString::new("baseline").unwrap().as_ptr()) } {
            -1 => return Err("Invalid Argument"),
            0 => (),
            _ => return Err("Unexpected"),
        }

        //* 编码需要的辅助变量
        let nb_nal: c_int = 0;
        let c_nals: *mut X264NalT = unsafe { mem::zeroed() };
        let mut pic_in: X264PictureT = unsafe { mem::zeroed() };
        let mut pic_out: X264PictureT = unsafe { mem::zeroed() };
        if unsafe {
            x264_picture_init(&mut pic_out as *mut X264PictureT);
            x264_picture_alloc(&mut pic_in as *mut X264PictureT,
                                param.i_csp,
                                param.i_width,
                                param.i_height)
        } < 0 {
            return Err("Allocation Failure");
        }
        pic_in.img.i_csp = X264_CSP_I420 as i32;
        pic_in.img.i_plane = 3;

        //* 打开编码器句柄,通过x264_encoder_parameters得到设置给X264
        //* 的参数.通过x264_encoder_reconfig更新X264的参数
        let enc = unsafe { x264_encoder_open(&mut param as *mut X264ParamT) };

        if enc.is_null() {
            return Err("Out of Memory");
        }
        let scale = scale_from_csp(param.i_csp as u32 & X264_CSP_MASK as u32);
        let bytes = 1 + (param.i_csp as u32 & X264_CSP_HIGH_DEPTH as u32);
        let mut plane_size = [0; 3];

        for i in 0..pic_in.img.i_plane as usize {
            plane_size[i] = param.i_width as usize * scale.w[i] / 256 * bytes as usize *
                            param.i_height as usize *
                            scale.h[i] / 256;
        }

        Ok(X264Encoder{
            plane_size: plane_size,
            nb_nal: nb_nal,
            c_nals: c_nals,
            pic_in: pic_in,
            pic_out: pic_out,
            current_frame: 0,
            enc: enc,
            width,
            height
        })
    }

    pub fn width(&self) -> i32{
        self.width
    }

    pub fn height(&self) -> i32{
        self.height
    }

    pub fn encode(&mut self, yu12_frame:Vec<u8>)->Result<Vec<u8>, &'static str>{
        
        let plan_slice_0 = unsafe{ slice::from_raw_parts_mut(self.pic_in.img.plane[0], self.plane_size[0]) };
        let plan_slice_1 = unsafe{ slice::from_raw_parts_mut(self.pic_in.img.plane[1], self.plane_size[1]) };
        let plan_slice_2 = unsafe{ slice::from_raw_parts_mut(self.pic_in.img.plane[2], self.plane_size[2]) };

        plan_slice_0.copy_from_slice(yu12_frame.get(..self.plane_size[0]).unwrap());
        plan_slice_1.copy_from_slice(yu12_frame.get(self.plane_size[0]..self.plane_size[0]+self.plane_size[1]).unwrap());
        plan_slice_2.copy_from_slice(yu12_frame.get(self.plane_size[0]+self.plane_size[1]..).unwrap());

        self.pic_in.i_pts = self.current_frame as i64;

        let frame_size = unsafe {
            x264_encoder_encode(self.enc,
                                &mut self.c_nals as *mut *mut X264NalT,
                                &mut self.nb_nal as *mut c_int,
                                &mut self.pic_in as *mut X264PictureT,
                                &mut self.pic_out as *mut X264PictureT)
        };
        
        if frame_size < 0 {
            return Err("Error encoding");
        }else{
            let mut data = vec![];
            for i in 0..self.nb_nal {
                let nal = unsafe { Box::from_raw(self.c_nals.offset(i as isize)) };

                let payload =
                    unsafe { slice::from_raw_parts(nal.p_payload, nal.i_payload as usize) };

                data.extend_from_slice(payload);

                mem::forget(payload);
                mem::forget(nal);
            }
            self.current_frame += 1;
            Ok(data)
        }
    }

    pub fn destroy(&mut self){
        println!("x264 destroy start...");
        unsafe { x264_encoder_close(self.enc) };
        unsafe { x264_picture_clean(&mut self.pic_in) };
        //unsafe { x264_picture_clean(&mut self.pic_out) };
        println!("x264 destroy send.");
    }

    pub fn frame(&self)->u32{
        self.current_frame
    }
}

impl Drop for X264Encoder{
    fn drop(&mut self) {
        self.destroy();
    }
}

struct ColorspaceScale {
    w: [usize; 3],
    h: [usize; 3],
}

fn scale_from_csp(csp: u32) -> ColorspaceScale {
    if csp == X264_CSP_I420 {
        ColorspaceScale {
            w: [256 * 1, 256 / 2, 256 / 2],
            h: [256 * 1, 256 / 2, 256 / 2],
        }
    } else if csp == X264_CSP_YV12 {
        ColorspaceScale {
            w: [256 * 1, 256 / 2, 256 / 2],
            h: [256 * 1, 256 / 2, 256 / 2],
        }
    } else if csp == X264_CSP_NV12 {
        ColorspaceScale {
            w: [256 * 1, 256 * 1, 0],
            h: [256 * 1, 256 / 2, 0],
        }
    } else if csp == X264_CSP_NV21 {
        ColorspaceScale {
            w: [256 * 1, 256 * 1, 0],
            h: [256 * 1, 256 / 2, 0],
        }
    } else if csp == X264_CSP_I422 {
        ColorspaceScale {
            w: [256 * 1, 256 / 2, 256 / 2],
            h: [256 * 1, 256 * 1, 256 * 1],
        }
    } else if csp == X264_CSP_YV16 {
        ColorspaceScale {
            w: [256 * 1, 256 / 2, 256 / 2],
            h: [256 * 1, 256 * 1, 256 * 1],
        }
    } else if csp == X264_CSP_NV16 {
        ColorspaceScale {
            w: [256 * 1, 256 * 1, 0],
            h: [256 * 1, 256 * 1, 0],
        }
        /*
    } else if csp == X264_CSP_YUYV {
        ColorspaceScale {
            w: [256 * 2, 0, 0],
            h: [256 * 1, 0, 0],
        }
    } else if csp == X264_CSP_UYVY {
        ColorspaceScale {
            w: [256 * 2, 0, 0],
            h: [256 * 1, 0, 0],
        }
        */
    } else if csp == X264_CSP_I444 {
        ColorspaceScale {
            w: [256 * 1, 256 * 1, 256 * 1],
            h: [256 * 1, 256 * 1, 256 * 1],
        }
    } else if csp == X264_CSP_YV24 {
        ColorspaceScale {
            w: [256 * 1, 256 * 1, 256 * 1],
            h: [256 * 1, 256 * 1, 256 * 1],
        }
    } else if csp == X264_CSP_BGR {
        ColorspaceScale {
            w: [256 * 3, 0, 0],
            h: [256 * 1, 0, 0],
        }
    } else if csp == X264_CSP_BGRA {
        ColorspaceScale {
            w: [256 * 4, 0, 0],
            h: [256 * 1, 0, 0],
        }
    } else if csp == X264_CSP_RGB {
        ColorspaceScale {
            w: [256 * 3, 0, 0],
            h: [256 * 1, 0, 0],
        }
    } else {
        unreachable!()
    }
}