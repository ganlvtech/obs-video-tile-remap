# OBS 视频分块重映射插件

《一种基于 UV 映射贴图的高清 OBS 直播解决方案》

将源分成 16x16 的小块，然后使用指定的种子产生随机序列，对画面进行打乱。

使用油猴脚本，输入相同参数，使用 WebGL 2 进行渲染来反向复原。

说明：16x16 是视频流压缩的最小单元，应该可以做到对码率影响最小。

支持局部编码，只对局部的画面进行编码，其他部分仍保持正常可见，规避低质量直播。

OBS 插件和 WebGL 均使用显卡进行渲染，仅需一个 Vertex Shader，一个 Fragment Shader，对显卡的占用很小，可以做到实时解码。

目前仅提供 Windows 版 OBS 插件。不过本插件是可以移植 MacOS 的，希望熟悉 MacOS 开发的人可以提供一些帮助。

## 使用方法

### 推流

1. 将 obs_video_tile_remap.dll 复制到 `C:\Program Files\obs-studio\obs-plugins\64bit`，然后重启 OBS 即可。

2. 在 OBS 中选择一个视频源或者场景，添加一个 Video Tile Remap 的滤镜，然后设置一个“密码”作为种子。

   说明：如果是全屏加密，可以将滤镜添加到场景上。

   参数说明：

   1. 第 1 个参数是随机数种子，可以使用文字或者是数字。（如果使用文字，请不要包含英文逗号）

      这个参数需要告诉观看者用于解码。

   2. 第 2 个和第 3 个参数是 UV 映射贴图的大小，需要与推流端对应。通常与源的宽高相同，通常 1920x1080 即可，不需要修改。

      这个参数与视频源的宽高并无必然联系，3840x2160 的视频源依然可以使用 1920x1080 的 UV 映射贴图。

      这个参数通常不会大于 OBS 最终输出的画面的宽高。因为 UV 映射贴图的大小超过输出画面大小也不会对画质有提升。

      这个参数需要告诉观看者用于解码。

   3. 第 4 个和第 5 个参数是 UV 映射贴图每个块的大小，需要与推流端对应。通常全屏编码可以设为 16x16。横向会被分成 1920 / 16 = 120 个小块，纵向会被分成 1080 / 16 = 67.5 个小块。

      这个参数需要告诉观看者用于解码。

   4. 第 6 个参数是编码区域，由多个矩形区域组成。这个参数可以留空表示全屏都参与编码。示例值：`[0,0,1920,80],[0,896,1440,1080]`，这会将画面打乱并编码到上方和左下方的一部分区域。

      需要注意，编码区域越小，还原出来的画面越糊。 

   5. 第 7 个参数是过渡的进度，从 0.0 ~ 1.0 对应每个小块从原来的位置平移到目标位置的中间状态，通常为 1.0。

      你还可以尝试使用一个较小的数值（例如 0.01）来使画面产生微小的位移直接进行直播，这样很可能会被识别出来。

### 观看

安装 Tampermonkey 浏览器插件 https://www.tampermonkey.net/ ，Chrome 浏览器和 Edge 浏览器都可以直接在商店进行安装。

然后访问 https://github.com/ganlvtech/obs-video-tile-remap/raw/main/userscripts/video_decode.user.js 安装视频解码脚本。

然后访问任意直播间，在右上角的插件中找到“obs-video-tile-remap 解码”

* 使用默认参数解码视频：这个就是使用默认参数 `0,1920,1080,16,16` 解码。
* 自定义参数解码视频：这个可以指定自定义密码，自定义区域，示例值如下：
  * `0,1920,1080,16,16`
  * `ganlvtech,1920,1080,16,16`
  * `0,1920,1080,16,16,[[0,0,1920,80],[0,896,1440,1080]]`
  * `0,1920,1080,16,16,[[64,0,1920,80],[64,896,1440,1072]]`

## 构建

1. 安装 Rust https://rustup.rs/

   安装时选择默认的 `x86_64-pc-windows-msvc` 工具链

2. 安装 MSVC 编译器（最新版的 Rust 在安装时会自动安装 Visual Studio 2022 生成工具）

   在 [Visual Studio 下载页面](https://visualstudio.microsoft.com/zh-hans/downloads/#build-tools-for-visual-studio-2022)
   下载`Visual Studio 2022 生成工具`

3. 安装 OBS Studio

   在 [OBS Releases 页面](https://github.com/obsproject/obs-studio/releases)
   下载 `OBS-Studio-29.1.3-Full-Installer-x64.exe` 并安装

4. 安装 bindgen-cli

   ```bash
   cargo install bindgen-cli
   ```

   注意，你需要将 `~/.cargo/bin` 添加到 PATH 环境变量。

   bindgen-cli 文档： https://rust-lang.github.io/rust-bindgen/command-line-usage.html

5. 安装 LLVM

   在 [LLVM Releases 页面](https://github.com/llvm/llvm-project/releases)下载 `LLVM-16.0.5-win64.exe`

   添加环境变量

   ```bash
   LIBCLANG_PATH=C:\Program Files\LLVM\bin
   ```

6. 下载本项目源码

   ```bash
   git clone https://github.com/ganlvtech/obs-video-tile-remap.git
   cd obs-video-tile-remap
   ```

7. 下载 OBS 源码

   ```bash
   git clone https://github.com/obsproject/obs-studio.git
   cd obs-studio
   git checkout 29.1.3
   cd ..
   ```

8. 生成 bindings.rs

   ```bash
   cd bindings/src/
   bindgen --with-derive-default wrapper.h -o bindings.rs
   ```

   此时项目的大概结构是

   ```plain
   obs-video-tile-remap
   |-- .cargo
   |-- bindings
   |   |-- src
   |   |   |-- bindings.rs
   |   |   |-- lib.rs
   |   |   \-- wrapper.h
   |   \-- Cargo.toml
   |-- obs-studio
   |   \-- libobs
   |       |-- util
   |       |   \-- platform.h
   |       \-- obs-module.h
   |-- src
   |   |-- lib.rs
   |   \-- uv_mapping.effect
   |-- userscripts
   |   \-- video_decode.user.js
   \-- uv_map
       |-- src
       |   \-- lib.rs
       \-- Cargo.toml
   ```

9. 编译

   ```bash
   cargo build --release
   ```

   然后就可以得到 target/release/obs_video_tile_remap.dll

## LICENSE

uv_map 和 webgl 的代码使用的是 MIT License。

build.rs 和 OBS 插件的代码是使用与 obs-studio 一样的 GPLv2 许可证。

插件二进制文件使用 GPLv2 许可证发布。
