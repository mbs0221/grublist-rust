# Grub-List (Rust版本)

这是用Rust重新实现的grub选择工具，功能与Python版本相同。

## 功能

* 以与grub相同可视化的界面设置下一次开机要启动的kernel版本。
* 可以用ssh远程调用脚本在关机前进行设置。

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

与grub界面相同的操作方法：
* 上下方向键选择entry
* 回车或右方向键选择启动项或打开子目录
* 左方向键收起子目录
* q退出grublist

## 项目结构

- `src/main.rs` - 主程序入口，包含菜单交互逻辑
- `src/grub.rs` - GRUB配置文件解析模块
- `src/colorprint.rs` - 颜色输出模块
- `src/interaction.rs` - 键盘输入处理模块

## 依赖

- `regex` - 用于解析grub.cfg文件
- `termion` - 用于终端交互和键盘输入

