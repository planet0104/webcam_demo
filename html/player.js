

var decoder = null;

//绘制h264图像blob
function redner_h264(context, blob){
    //console.log('redner_h264', blob);
    var reader = new FileReader();
    reader.onload = function(readerEvent){
        decode_h264(context, new Uint8Array(readerEvent.target.result));
    };
    reader.readAsArrayBuffer(blob);
}

//解码并显示h264 data: Uint8Array
function decode_h264(context, data){
    //console.log('长度:', data.length);
    if(decoder == null){
        decoder = new Decoder();
        decoder.canvas_context = context;
        decoder.canvas_buffer = {};

        decoder.onPictureDecoded = function (buffer, width, height) {
            if(!decoder.canvas_buffer[width+'x'+height]){
                decoder.canvas_buffer[width+'x'+height] = context.createImageData(width, height);
            }
            if (!buffer){
                console.log("buffer is null!");
                return;
            }
            if(canvas.width != width){
                canvas.width = width;
                canvas.height = height;
            }
        
            var lumaSize = width * height;
            var chromaSize = lumaSize >> 2;
        
            var ybuf = buffer.subarray(0, lumaSize);
            var ubuf = buffer.subarray(lumaSize, lumaSize + chromaSize);
            var vbuf = buffer.subarray(lumaSize + chromaSize, lumaSize + 2 * chromaSize);
        
            for (var y = 0; y < height; y++) {
                for (var x = 0; x < width; x++) {
                    var yIndex = x + y * width;
                    var uIndex = ~~(y / 2) * ~~(width / 2) + ~~(x / 2);
                    var vIndex = ~~(y / 2) * ~~(width / 2) + ~~(x / 2);
                    var R = 1.164 * (ybuf[yIndex] - 16) + 1.596 * (vbuf[vIndex] - 128);
                    var G = 1.164 * (ybuf[yIndex] - 16) - 0.813 * (vbuf[vIndex] - 128) - 0.391 * (ubuf[uIndex] - 128);
                    var B = 1.164 * (ybuf[yIndex] - 16) + 2.018 * (ubuf[uIndex] - 128);
                    
                    var rgbIndex = yIndex * 4;
                    this.canvas_buffer[width+'x'+height].data[rgbIndex+0] = R;
                    this.canvas_buffer[width+'x'+height].data[rgbIndex+1] = G;
                    this.canvas_buffer[width+'x'+height].data[rgbIndex+2] = B;
                    this.canvas_buffer[width+'x'+height].data[rgbIndex+3] = 0xff;
                }
            }
        
            this.canvas_context.putImageData(this.canvas_buffer[width+'x'+height], 0, 0);
            //var date = new Date();
            //console.log("WSAvcPlayer: Decode time: " + (date.getTime() - this.rcvtime) + " ms");
        };
    }

    //decoder.rcvtime = new Date().getTime();
    var naltype = "invalid frame";
    //console.log(data, 'data.length=', data.length);
    if (data.length > 4) {
      if (data[4] == 0x65) {
        naltype = "I frame";
      }
      else if (data[4] == 0x41) {
        naltype = "P frame";
      }
      else if (data[4] == 0x67) {
        naltype = "SPS";
      }
      else if (data[4] == 0x68) {
        naltype = "PPS";
      }
    }
    //console.log("Passed " + naltype + " to decoder");
    decoder.decode(data);
}