# 发布到 crates.io 指南

## 准备工作

### 1. 更新 Cargo.toml

在发布之前，请更新 `Cargo.toml` 中的以下字段：

- `authors`: 将 `"Your Name <your.email@example.com>"` 替换为你的真实信息
- `repository`: 将 `"https://github.com/yourusername/grublist-rust"` 替换为你的GitHub仓库地址（如果有的话）

### 2. 注册 crates.io 账号

1. 访问 https://crates.io
2. 使用 GitHub 账号登录
3. 获取你的 API token：
   - 登录后访问 https://crates.io/me
   - 点击 "New Token"
   - 设置 token 名称（如 "grublist-publish"）
   - 复制生成的 token

### 3. 配置 cargo 认证

```bash
cargo login <your-api-token>
```

## 发布步骤

### 1. 验证包内容

```bash
# 检查包是否可以正常打包
cargo package

# 查看将要发布的文件列表
cargo package --list
```

### 2. 检查包内容

```bash
# 解压并检查包内容（可选）
cd target/package/grublist-0.1.0
# 检查文件是否正确
```

### 3. 发布到 crates.io

```bash
# 发布（会自动执行 cargo package）
cargo publish
```

**注意**: 发布后无法删除或覆盖版本，只能发布新版本。

## 发布新版本

当你需要发布新版本时：

1. 更新 `Cargo.toml` 中的 `version` 字段（遵循语义化版本控制）
2. 运行 `cargo publish` 发布新版本

版本号规则：
- `0.1.0` → `0.1.1` (补丁版本，bug修复)
- `0.1.0` → `0.2.0` (次版本，新功能，向后兼容)
- `0.1.0` → `1.0.0` (主版本，不兼容的更改)

## 安装已发布的包

发布成功后，其他人可以通过以下方式安装：

```bash
# 安装二进制程序
cargo install grublist

# 或作为依赖添加到 Cargo.toml
[dependencies]
grublist = "0.1.0"
```

## 常见问题

### 包名已被占用

如果 `grublist` 这个名字已被占用，你需要：
1. 在 `Cargo.toml` 中更改 `name` 字段
2. 确保新名字符合 crates.io 的命名规则

### 发布失败

- 检查网络连接
- 确认 API token 有效
- 检查是否有编译错误或警告
- 确保所有必需的文件都在包中

## 有用的命令

```bash
# 检查包是否可以发布（不实际上传）
cargo publish --dry-run

# 查看包的元数据
cargo metadata

# 检查代码质量
cargo clippy
cargo fmt --check
```

