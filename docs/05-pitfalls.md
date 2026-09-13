# 已知坑（踩过的，别再踩）

> 从 Agent.md 拆出的踩坑档案，[Agent.md](../Agent.md) 指向此处。排障、写系统级代码、
> 或遇到「改了没生效」时先过一遍；新踩的坑按编号追加，写明症状 → 根因 → 修法/规避并注日期，
> 同时评估是否要回写 Agent.md 的流程或红线。

1. **vite 依赖重优化会触发整页重载**（lockfile/deps 变化后首次 dev）：表现为路由重置、点击落空、
   「按钮没反应」。先怀疑重载竞态，等几秒重来一次再排查代码。
2. **强杀 tauri dev / cargo 后可能跑旧二进制**（cargo 指纹 mtime 竞态）：杀完进程先 `cargo build` 再起，
   并核对 `target/debug/hamster-hub.exe` 时间戳。
3. **WebView2 的 a11y 树是陈旧缓存**：DOM 内容变了树不更新。验证 UI 状态用截图，别信 a11y。
4. **桌面壳里不要用 alert() 调试**（可能阻塞异步链），dev 版已启用 devtools（F12），优先 console。
5. **合成点击坐标在桌面壳里不可靠**：UI 验证一律在第 1 层做；第 2 层用程序断言（PowerShell/文件/进程）。
6. **vite watch 必须忽略非源码目录**（已配：`src-tauri/e2e/scripts/test-results`）：
    cargo 写 target 里的 dll 会 EBUSY 崩 vite；e2e/scripts 文件变动会触发无谓重载甚至进程退出。
7. **本机网络**：git/npm 全局有死代理 `127.0.0.1:31181`（已用项目 `.npmrc` 覆盖）；cargo 需要
   `NO_PROXY='*'` 前缀；https 偶发吊销检查离线 → curl 加 `--ssl-no-revoke`，cargo 用 `.cargo/config.toml`
   的 `check-revoke=false`（已配）。
8. **测试二进制必须嵌 manifest**（build.rs 已做）：comctl32 v6 缺失会 STATUS_ENTRYPOINT_NOT_FOUND。
9. **第三方常驻软件可能提权占用任务栏**：测系统接管前确认任务栏状态基线（IsWindowVisible），别拿截图肉眼判断。
10. **Alt+Space 是 Windows 系统菜单保留键**，RegisterHotKey 大概率失败：已做容错注册（日志告警 +
    托盘菜单兜底）；Spotlight 默认热键后续可配。
11. **看门狗判活必须校验进程镜像名**（QueryFullProcessImageNameW 包含 hamster-hub）：裸 OpenProcess
    会把「死后被系统复用的 PID」判活，导致崩溃后永不还原（真机踩过）。
12. **主进程死后 stdio 管道即断**：子进程（看门狗）里任何 eprintln 都会 broken-pipe panic——
    看门狗日志一律写 `%APPDATA%\com.hamsterhub.app\watchdog.log`。
13. **tasklist //FI 有时序性延迟**：断言进程存在/退出用 `tasklist | grep`（全量）或 PowerShell
    `Get-Process`，且提取 PID 别用内存数字尾巴。
14. **tauri dev 的 file watcher 会在源码变更时重启 app**：正在跑系统矩阵时不要改 Rust 源码；
    杀主进程测试会让 `pnpm tauri dev` 整体退出（exit 101/1），属预期。
15. **多窗口（main/spotlight）共享 QueryClient 配置**：`refetchOnWindowFocus` 不能关——
    Spotlight 隐藏期间首屏查询失败/为空会被 `staleTime` 缓存，窗口重显靠 focus 重取救回。
16. **FTS5 unicode61 分词**：CJK 连续串是单 token（"微信截图"整体），中文子串检索必须走拼音列；
    拼音列存双形态（`wei xin jie tu` + `weixinjietu`）分别支持音节 AND 与整词前缀。
17. **纯 ASCII 名（Visual Studio Code）没有拼音音节**，首字母缩写（vsc）需在 pinyin_cols 里
    单独提取「词首字母」追加进全拼列与首字母列，否则缩写检索永远空。
18. **tauri bundle resources 不接受 `../` 逃逸路径**（build.rs 阶段就报 doesn't exist）：
    外部构建产物（watchdog exe）先复制到 `src-tauri/resources/` 再以相对路径声明。
19. **主窗口禁止 `transparent: true` + `apply_acrylic`**（2026-09-11 真机踩坑）：WebView2 透明合成
    会把整窗渲染成"透出背后内容"的半透明灰（随背后窗口颜色变化，用户看就是配色全乱），
    且 DOM 实底背景色也压不住。主窗口保持不透明，深浅色观感全靠 CSS 背景承担；
    透明 + 毛玻璃只用于 Spotlight 覆盖层窗口。另：`bg-[var(--bg)]` 这类 Tailwind 任意值
    在壳内不可靠时，用内联 `style={{ background: 'var(--bg, #17151b)' }}` 兜底。
20. **"改了没生效"先查是不是旧实例**：single-instance 插件会让新进程退出并唤起旧窗口——
    打过 NSIS 包后 `target/release/` 的旧 exe 若在运行，`tauri dev` 的新 debug 实例永远起不来。
    用 `Get-Process -Id <pid> | Select Path,StartTime` 查运行实例的来源与启动时间，
    `taskkill` 全部 hamster 进程后再启动。
21. **useNavigate 的 navigate 标识随 location 变化，不能作为「含导航副作用」effect 的依赖**
    （2026-09-12 踩坑，浏览器程序化定位）：`[navigate]` 依赖会让 effect 在**每次路由变化时重跑**；
    若 effect 里做「查状态→强制 navigate」（如 desktopModeIsActive→/desktop），用户的一切导航
    都会在 ~3ms 内被 replace 弹回，表现为**页面所有按钮点击"无效"**（事件已到 DOM、navigate 已执行、
    又被回滚）。症状与坑 1（vite 重载竞态）高度相似，用 pushState 补丁时间线区分。
    修法（AppShell）：初始对齐 `[]` 依赖 + `window.location.hash` 守卫只跑一次；事件监听 effect 用
    alive 标志兜底「注销函数晚于卸载 resolve」造成的 StrictMode 重复监听。
22. **桌面接管的恢复链路必须两条腿：看门狗 + 启动对账**（2026-09-13 补缺口，对标竞品反推）：
    看门狗只覆盖「主进程死了但它活着」的窗口；断电/蓝屏（双进程同死）、watchdog exe 缺失被跳过、
    watchdog 被杀或 24h 自杀退出（现行为：超时不恢复直接退）都会留下「系统隐藏态 + 残留快照」。
    更糟的是污染：下次 enter() 会把隐藏态拍成「原始状态」存进新快照，退出时忠实还原到隐藏——
    副屏任务栏与桌面图标（HideIcons=1 被回放）永久消失，仅主任务栏有按类名兜底可救。
    修法（desktop_mode::reconcile_stale_snapshot）：app 启动（setup）与 enter() 拍快照前各对账一次，
    快照存在即无条件恢复并删除，损坏时无差别恢复（可见桌面优于黑屏）。
    遗留：watchdog 24h 自杀保护仍是「不恢复直接退」，有启动对账兜底、下次启动自愈，暂不改。
    经验：恢复逻辑不能只活在「某进程还活着」的前提里；落盘快照的另一个读者必须是下一次启动。
23. **cargo 拉依赖挂死：crates.io 被墙 → schannel 吊销检查离线 → dev-sidecar 系统代理黑洞**（2026-09-13，接 updater 插件首次拉取新依赖时踩，三层叠加）：
    症状：任意 cargo 命令报 `download of config.json failed: transfer too slow (0 bytes in 30s)`；
    换 rsproxy 镜像后仍挂（0 CPU 无 rustc、curl 却秒通）；并行会话的 cargo 全部互等构建/包缓存锁。
    排查链（每层都可能单独坑）：
    ① index.crates.io 直连被墙 → `~/.cargo/config.toml` 配 rsproxy sparse 镜像；
    ② cargo（libcurl+schannel）做证书吊销检查而 CRL/OCSP 不可达 → `[http] check-revoke = false`；
    ③ **dev-sidecar 把 Windows 系统代理设为 127.0.0.1:31181，cargo 会读注册表 ProxyServer 但不理会
    ProxyOverride 白名单**，请求全进 dev-sidecar 而它对 rsproxy.cn 黑洞（连接 ESTABLISHED 但 0 字节）。
    修法：`setx NO_PROXY rsproxy.cn,.rsproxy.cn,index.crates.io,static.crates.io,...`（env 层短路，已设置）。
    诊断口诀：`Get-NetTCPConnection -RemotePort 31181` 看谁挂在代理上；cargo 全家 0 CPU 无 rustc
    = 有人在等锁/等网络，先 `Get-Process cargo | Stop-Process -Force` 清场再重试。失败重试前不清
    孤儿进程 = 越积越多互相锁死。CI（GitHub Actions）无此问题。
24. **IPC mock 的载荷字段名会悄悄与生成契约脱钩**（2026-09-13 做通知中心时发现）：
    `ipc-mock.ts` 里的 `mockEvent<T>` 是手写泛型，而 `mockEvents` 整体被 `as typeof realEvents` 断言过，
    T 写错**编译器不会报**。实际中 `benchStreamExit` mock 发 `{ sessionId }`、生成类型（Rust 侧
    `session_id`）与消费方 FloatingAgent 读的都是 `session_id` → 浏览器层里 `e.payload.session_id`
    恒为 undefined，该兜底收口静默失效（`benchPtyExit` 同病：`exitCode` vs `exit_code`）。
    症状特征：只读 payload 的消费方「在真机好、在浏览器预览没反应」。
    修法：mock 的载荷字段名一律**照抄 `src/shared/types/ipc.ts` 生成类型**（snake_case），
    新增事件时先打开生成类型对照；只 `listen(() => …)` 不读 payload 的消费方会掩盖问题，别据此判断 mock 正确。
