# 安全设计

## 网页版

页面和主页同源，所以首要目标是选手代码碰不到这个源。网站是纯静态的，没有后端、账号、密钥和上传接口。选手代码只在访问者自己的浏览器里运行。

| 威胁 | 措施 | 测试 |
| --- | --- | --- |
| 读取主页的存储、Cookie、DOM | 选手在 `sandbox="allow-scripts"` 的 iframe 里运行，源是不透明的，代码又在 Worker 里，没有 DOM | `attack js reads the site` |
| 外传数据、借访问者的网络发请求 | JS 和 WASM 沙箱的 CSP 是 `connect-src 'none'`。Python 和编译器沙箱只能访问固定版本的 CDN 目录 | 同上，`attack python escapes` |
| 直接打开沙箱页面，绕过隔离 | 沙箱页检查 `self.origin`，不是不透明源就拒绝启动 | `attack sandbox page` |
| 死循环、刷消息 | 每回合有时限，超时销毁 iframe。消息数、日志量、单条长度都有上限 | `attack endless loop`、`attack message flood` |
| 伪造消息 | 主页面核对消息来源窗口、类型和回合号，内容由 Rust 内核再校验 | `attack spoofed` |
| 用选手名、日志、回放文件注入脚本 | 文本只用 `textContent` 显示，回放文件校验结构和大小，主页面 CSP 禁止内联脚本和 eval | `attack markup`、`attack inline script` |
| CDN 被篡改 | 版本固定，CDN 代码只在沙箱里加载，主页面不加载任何第三方脚本 | 沙箱 CSP |

残留风险：选手代码可以占用访问者自己的 CPU 和内存，直到超时或关闭页面。GitHub Pages 不能设置响应头，因此没有 `frame-ancestors`，其他网站可以嵌入本页面。

修改时必须遵守：

1. 不给沙箱 iframe 加 `allow-same-origin`。
2. 主页面的 `script-src` 不加 `'unsafe-inline'`、`'unsafe-eval'` 或外部域名。
3. 不用 `innerHTML` 显示任何来自选手、回放或反馈的内容。
4. 不把令牌或密钥放进前端。
5. 不从网址参数自动加载并运行代码。

## 反馈

页面不收集数据。提交时打开预填的 GitHub Issue，由用户自己确认，反滥用由 GitHub 处理。以后要做站内收集，应使用独立域名的服务，并加上限流、长度限制、人机验证和审核后再公开。

## 本地版

`npe run`、`npe tournament` 和 `npe studio` 不是安全沙箱，选手以当前用户的权限运行。引擎只负责超时、崩溃隔离、进程树清理，并且不把宿主的环境变量传给选手。

`npe studio` 只监听 127.0.0.1，校验 Host、Origin 和随机令牌，其他网页无法借它执行代码。

## 以后做在线天梯

服务端运行陌生人的代码是另一个量级的风险，不能直接用本地版的 runner。至少要做到：评测机和网站分离且不放任何凭据，每个选手一个无网络、只读、限制 CPU、内存、进程数的容器或虚拟机，编译也在容器里进行，评测结果通过单向队列返回。
