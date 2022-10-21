# tinyproxy-rust
A tiny proxy write with rust

1，支持日志记录，日志放在  tinyproxy.log

2, 增加代理认证功能，Proxy-Authorization，默认用户名  tzf 密码1234
可以在 let  authcode=base64::encode(b"tzf:1234"); 行修改为自己的

3,默认访问端口 8010

