# Grub-List (Rust版本)

这是用Rust重新实现的grub选择工具，功能与Python版本相同，并增加了kernel参数配置功能。

## 功能

* 以与grub相同可视化的界面设置下一次开机要启动的kernel版本。
* 可以用ssh远程调用脚本在关机前进行设置。
* **新增**：交互式配置kernel启动参数（GRUB_CMDLINE_LINUX 和 GRUB_CMDLINE_LINUX_DEFAULT）
  - 列表式参数管理，逐个设置参数值
  - 支持添加、编辑、删除参数
  - 自动创建配置文件备份
* **新增**：Kernel版本信息显示
  - 查看每个启动项对应的kernel版本、架构和文件路径
* **新增**：清理旧Kernel版本
  - 扫描未使用的kernel版本
  - 安全删除旧kernel文件以释放磁盘空间
* **新增**：启动项重命名
  - 为启动项设置自定义名称
  - 让启动项更易识别和管理
* **新增**：配置文件备份管理
  - 查看所有配置文件备份
  - 恢复或删除备份文件
* **新增**：GRUB配置验证
  - 验证GRUB配置文件语法
  - 检查配置错误和警告

## 编译

```bash
cargo build --release
```

编译后的可执行文件位于 `target/release/grublist`

## 使用方法

```bash
./target/release/grublist
```

或者安装到系统路径：

```bash
cargo install --path .
```

然后直接运行：

```bash
grublist
```

## 操作说明

### 基本导航

与grub界面相同的操作方法：
* 上下方向键选择entry（支持循环滚动）
* 回车或右方向键选择启动项或打开子目录
* 左方向键返回上一级菜单
* ESC键退出到上级菜单
* 鼠标点击菜单项进行选择
* q退出grublist

### Kernel参数配置

在主菜单底部选择 `[⚙] Configure Kernel Parameters` 进入配置界面：

1. **选择配置项**：
   - 选择 `1` 编辑 `GRUB_CMDLINE_LINUX`
   - 选择 `2` 编辑 `GRUB_CMDLINE_LINUX_DEFAULT`

2. **参数列表管理**：
   - 使用上下方向键选择参数项（支持循环滚动）
   - 回车或右方向键进入编辑模式
   - 编辑参数值时，右方向键保存并自动进入下一个参数的编辑
   - 回车键保存并退出编辑模式
   - 左方向键返回上一级菜单
   - ESC键退出到上级菜单

3. **编辑参数值**：
   - 对于有值的参数（如 `acpi=off`），只编辑值部分（`off`）
   - 对于标志参数（如 `quiet`），可以添加值或编辑名称
   - 保存后需要运行 `sudo update-grub` 应用更改

### 示例

假设当前kernel参数为：`quiet splash acpi=off`

在参数列表界面会显示：
```
Parameters:
  1. quiet
  2. splash
  3. acpi=off
```

- 编辑 `3`：输入新值 `on`，参数变为 `acpi=on`
- 添加参数：输入 `a`，然后输入参数名 `nomodeset`，再输入值（或留空），添加成功
- 删除参数：输入 `d`，然后输入要删除的参数编号

## 项目结构

- `src/main.rs` - 主程序入口，包含菜单交互逻辑
- `src/grub.rs` - GRUB配置文件解析模块
- `src/grub_config.rs` - GRUB配置编辑模块（kernel参数配置）
- `src/colorprint.rs` - 颜色输出模块
- `src/interaction.rs` - 键盘输入处理模块

## 依赖

- `regex` - 用于解析grub.cfg文件和GRUB配置文件
- `ratatui` - 用于构建现代化的终端用户界面（TUI）
- `crossterm` - 用于跨平台终端操作和事件处理
- `serde` / `serde_json` - 用于数据序列化和自定义名称存储
- `chrono` - 用于时间格式化和备份时间显示

## 注意事项

* 修改kernel参数需要root权限
* 配置文件修改前会自动创建备份（`/etc/default/grub.bak`）
* 修改参数后需要运行 `sudo update-grub` 使更改生效
* 确保 `/etc/default/grub` 中的 `GRUB_DEFAULT=saved` 才能使用启动项选择功能

