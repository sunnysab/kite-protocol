# kite-protocol

上应小风筝 [**服务端**](https://github.com/sunnysab/kite-server) 与 [**代理端**](https://github.com/sunnysab/kite-agent) 协议的 Rust 实现。

注意：由于原项目未实现应用层分片传输，单个包大小受到严重限制，本项目已弃用。新的方案采用 WebSocket 作为底层协议。

## 文档

使用场景和概括性说明，请参阅服务端文档中的 [通信设计](https://github.com/sunnysab/kite-server/tree/master/docs/通信设计.md)  
实现细节说明见文档注释和 `src/bin` 目录下的示例代码。

## 开源协议

[GPL v3](https://github.com/sunnysab/kite-server/blob/master/LICENSE) © 上海应用技术大学易班 sunnysab  
除此之外，您也不能将本程序用于各类竞赛、毕业设计、论文等。
