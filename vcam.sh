sudo modprobe v4l2loopback
v4l2-ctl --list-device

# ffmpeg -loop 1 -re -i ./images/Chapter61_P22_P23_4k.jpg -f v4l2 -vcodec rawvideo -pix_fmt yuv420p /dev/video0

# 4000x3000
# 1280x720

ffmpeg -re -i ./mov.mp4 -f v4l2 -vcodec rawvideo -pix_fmt yuv420p /dev/video0

#Dummy video device (0x0000) (platform:v4l2loopback-000):
#        /dev/video0

# ffmpeg -re -i testsrc.avi -f v4l2 /dev/video1

# https://www.onlinemictest.com/webcam-test/