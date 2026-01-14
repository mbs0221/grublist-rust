# Grub-List (Rust版本)

这是用Rust重新实现的grub选择工具，功能与Python版本相同，并增加了kernel参数配置功能。

## 功能

* 以与grub相同可视化的界面设置下一次开机要启动的kernel版本。
* 可以用ssh远程调用脚本在关机前进行设置。
* **新增**：交互式配置kernel启动参数（GRUB_CMDLINE_LINUX 和 GRUB_CMDLINE_LINUX_DEFAULT）
  - 列表式参数管理，逐个设置参数值
  - 支持添加、编辑、删除参数
  - 自动创建配置文件备份

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
* 上下方向键选择entry
* 回车或右方向键选择启动项或打开子目录
* 左方向键收起子目录
* q退出grublist

### Kernel参数配置

在主菜单底部选择 `[⚙] Configure Kernel Parameters` 进入配置界面：

1. **选择配置项**：
   - 选择 `1` 编辑 `GRUB_CMDLINE_LINUX`
   - 选择 `2` 编辑 `GRUB_CMDLINE_LINUX_DEFAULT`

2. **参数列表管理**：
   - 输入数字（1-N）编辑对应参数的值
   - 输入 `a` 添加新参数
   - 输入 `d` 删除参数
   - 输入 `s` 保存并返回
   - 输入 `c` 取消

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
- `termion` - 用于终端交互和键盘输入

## 注意事项

* 修改kernel参数需要root权限
* 配置文件修改前会自动创建备份（`/etc/default/grub.bak`）
* 修改参数后需要运行 `sudo update-grub` 使更改生效
* 确保 `/etc/default/grub` 中的 `GRUB_DEFAULT=saved` 才能使用启动项选择功能

