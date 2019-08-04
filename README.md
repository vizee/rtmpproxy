# rtmpproxy

rtmpproxy 通过修改 RTMP 命令(例如 connect 和 publish)中的一些信息, 实现转发 RTMP 流到其他流服务器.

## 结合 dnsproxy 实现 PS4 twitch 流转发

主要思路就是利用 DNS 欺骗 PS4 把发往 twtich 的流转发到 rtmpproxy 上, 再由 rtmpproxy 转发到目标流 URL.

### 1. 首先你要先配置好 PS4 上的 twitch 直播

略.

### 2. PC 运行 dnsproxy

主要是配置 dnsproxy 要劫持的域名到 rtmpproxy 监听的 IP

### 3. 自定配置 PS4 网络

只需要修改 DNS 到 dnsproxy 监听的 IP, 其他配置不动.

### 4. 运行 rtmpproxy

启动参数 `-p` 传入直播完整推流URL+名称(例如: *rtmp://host/app?streamname=live*)

### 注意事项

1. 如果不知道要劫持什么域名, dnsproxy 开 `-V` 日志输出一下就知道了 :)
2. dnsproxy 要同时监听 udp 和 tcp
3. 对于 Windows, dnsproxy 编译时候可以直接删掉 USR 信号
4. 对于 Windows, 手工创建一个 resolve.conf
5. rtmpproxy 在 Linux 下可以获得 splice buff 加成
6. 由于 rtmpproxy 在处理 `publish` 后直接转 io.Copy, 所以 `publish` 后的命令无法替换 streamname
