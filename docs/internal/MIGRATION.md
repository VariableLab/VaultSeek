# 数据迁移与版本升级策略 (MIGRATION.md)

## 1. 版本号管理规范
- 使用 `SemVer` 规范: `主版本.次版本.补丁` (v1.2.0)。
- `turi.conf.json` 中的 `version` 必须与 git tag 同步。

## 2. 数据库迁移 (Database Schema Migration)
当 SQLite 表结构变更时：
1. **版本锁定**：在 `config` 表中定义 `schema_version`。
2. **迁移任务**：
    - 应用启动时，读取 `schema_version`。
    - 若 `version` 不匹配，执行 `migrations/` 下的对应 Rust 迁移脚本。
    - 迁移必须支持“快照备份”，防止迁移失败导致用户知识库丢失。

## 3. 嵌入模型迁移
当更换 Embedding 模型（导致向量长度变化）时：
- 不能直接混合存储。
- 策略：必须触发全库重扫 (Re-index)，或保留多个 `chunks_v1`, `chunks_v2` 表供过渡。
