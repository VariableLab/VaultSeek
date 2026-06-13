# 安全与供应链管理规范 (SECURITY.md)

## 1. 供应链安全 (Supply Chain Security)
- **定期审计**：每两周执行一次 `cargo audit`，检查依赖项中是否存在已知的 CVE 漏洞。
- **依赖引入门槛**：任何包含网络通信、密码学计算的新依赖，必须经过合伙人联合 Review。

## 2. 敏感信息存储 (Secret Management)
- **物理安全**：严禁在源代码、配置文件或日志中包含任何硬编码的 API Key。
- **运行时安全**：
    - API Key 必须使用 `keyring` 库，加密存储于操作系统提供的原生凭据管理器中（macOS Keychain / Windows DPAPI）。
    - 应用程序内存中的 API Key 引用在不再使用时应尽可能快速清零（Zeroization）。

## 3. 防逆向保护
- 发布正式版本时，Cargo profile 必须开启：
    - `strip = true` (剥离符号)
    - `lto = true` (链接时优化)
    - 确保二进制文件难以被轻易进行静态分析和逆向。
