// The MIT License (MIT)
//
// Copyright (c) 2023 Ganlv
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use std::cmp::min;

/// 标准伪随机数算法
pub fn prng(state: &mut u32) -> u32 {
    let new_state = state.wrapping_mul(1103515245).wrapping_add(12345);
    *state = new_state;
    new_state & 0x7fffffff
}

/// 标准 Hash 算法
pub fn hashcode(s: &[u8]) -> u32 {
    let mut h: u32 = 0;
    for x in s {
        h = h.wrapping_mul(31) + (*x) as u32;
    }
    h
}

/// 字符串转种子
pub fn string_to_seed(s: &[u8]) -> u32 {
    if s.len() > 10 {
        return hashcode(s);
    }
    let mut res: u64 = 0;
    for x in s {
        if *x >= b'0' && *x <= b'9' {
            res = res * 10 + (*x - b'0') as u64;
        } else {
            return hashcode(s);
        }
    }
    if res > 0xffffffff {
        return hashcode(s);
    }
    return res as u32;
}

/// 标准洗牌算法
pub fn shuffle<T>(list: &mut [T], seed: u32) {
    let mut state = seed;
    let len = list.len();
    for i in 0..len {
        let r = prng(&mut state) as usize;
        list.swap(i, i + r % (len - i));
    }
}

/// 解析 "[0,0,1920,100],[0,100,200,800],[1600,100,1920,800],[0,800,1920,1080]" 这样的字符串
pub fn parse_regions(s: &str) -> Result<Vec<(usize, usize, usize, usize)>, String> {
    let mut result = Vec::new();
    for s2 in s.split(']') {
        let s3 = s2.trim().trim_start_matches(',').trim().trim_start_matches('[');
        if s3.is_empty() {
            continue;
        }
        let mut iter = s3.split(',');
        match (iter.next(), iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), Some(c), Some(d)) => {
                match (a.trim().parse::<usize>(), b.trim().parse::<usize>(), c.trim().parse::<usize>(), d.trim().parse::<usize>()) {
                    (Ok(a), Ok(b), Ok(c), Ok(d)) => {
                        result.push((a, b, c, d));
                    }
                    _ => {
                        return Err(format!("parse number error. {}", s3));
                    }
                }
            }
            _ => {
                return Err(format!("elements not enough, at least 4 elements. {}", s3));
            }
        }
    }
    Ok(result)
}

/// 生成 UV 映射贴图（每 16 像素一个小块打乱）
///
/// * `regions`: `&[(left, top, right, bottom)]`: 编码图像的区域，注意不要重叠，请尽量将每个区域的上下左右都设置成 16 的倍数。
///
/// 首先把编码区域切分成 `cell_size_x` x `cell_size_y` 个小块
///
/// 例如：原始图像 1920x1080，显示区域是 `[(0, 0, 1920, 100), (0, 100, 200, 800), (1600, 100, 1920, 800), (0, 800, 1920, 1080)]`，这样的结果，中心部分就是透明的，四周是编码区域
///
///     x = 0  x = 200        x = 1600     x = 1920
///     |----------------------------------| y = 0
///     |                                  |
///     |----------------------------------| y = 100
///     |      |              |            |
///     |      |              |            |
///     |----------------------------------| y = 800
///     |                                  |
///     |----------------------------------| y = 1080
///
/// 每个编码区域单独计算 cell_count，然后把所有区域的 cell_count 加在一起，构成数组
///
/// 比如第一个 宽 1920 高 100 的区域，计算方法是这样的
/// 1920 / 16 = 120，
/// 100 / 16 = 6.25 向上取整到 7
///
/// 注意：
/// y = 0 ~ 100 部分的分块规则是这样的  0 - 16 - 32 - 48 - 64 - 80 - 96 - 100。
/// y = 100 ~ 800 部分的分块规则是这样的  100 - 112 - 128 - 144 - ... - 768 - 784 - 800，分块都会对齐 16 的整数倍，所以区域边缘的方块可能不是 16x16 的。
///
/// 得到的 cell_count 是这样的
///
///     cell_count = 120 * 7 + 13 * 44 + 20 * 44 + 120 * 18 = 4452
///
/// 然后使用 `seed` 进行 shuffle
///
/// 将 cell_count 按照原图像的 width:height 的比例求出分割的小块的大小，最后一横排部分的小块可能直接浪费掉
///
/// 例如 1920x1080 是 16:9
///
///     每个方格的大小是 1080 / sqrt(4452 * 9 / 16) = 21.58 向上取整到 22
///     1920 / 22 = 87.27 向上取整到 88
///     1080 / 22 = 49.09 向上取整到 50
///     88 * 50 = 4400
///
/// 4400 没有超过 4452，因此可以完整塞到编码区域中。
/// 如果超过了，则需要调大格子大小，让原画面分割的格子数小于编码后区域的格子数
///
/// `progress`: 过度的进度，0 表示原始状态，1 表示完全映射之后的状态
///
/// returns: `Vec<(f32, f32, f32, f32)>` RGBA32F 贴图，
/// R 通道是 U 0.0 ~ 1.0 范围，
/// G 通道是 V 0.0 ~ 1.0 范围，
/// B 通道始终是 0.0，
/// A 通道编码区域是 1.0，未编码区域是 0.0
pub fn generate_uv_map_texture(seed: u32, width: usize, height: usize, cell_size_x: usize, cell_size_y: usize, regions: &[(usize, usize, usize, usize)], progress: f32) -> Vec<(f32, f32, f32, f32)> {
    // 将编码区域划分网格
    let mut region_cells = Vec::with_capacity((width / cell_size_x + 1) * (height / cell_size_y + 1));
    for region in regions {
        let (left, top, right, bottom) = *region;
        let mut y0 = top;
        loop {
            let y1 = (y0 / cell_size_y + 1) * cell_size_y;
            let y2 = min(y1, bottom);
            let mut x0 = left;
            loop {
                let x1 = (x0 / cell_size_x + 1) * cell_size_x;
                let x2 = min(x1, right);
                region_cells.push((x0, y0, x2, y2));
                if x1 >= right {
                    break;
                }
                x0 = x2;
            }
            if y1 >= bottom {
                break;
            }
            y0 = y2;
        }
    }

    // 将原始视频区域划分网格
    let vertical_cell_count = (region_cells.len() as f32 * height as f32 / width as f32 * cell_size_x as f32 / cell_size_y as f32).sqrt();
    let mut image_cell_size_y = (height as f32 / vertical_cell_count).ceil() as usize;
    let mut image_cell_size_x = (image_cell_size_y as f32 * cell_size_x as f32 / cell_size_y as f32).ceil() as usize;
    // 如果格子不够的话，那么需要将调整格子大小，并浪费一些格子
    if ((width + image_cell_size_x - 1) / image_cell_size_x) * ((height + image_cell_size_y - 1) / image_cell_size_y) > region_cells.len() {
        image_cell_size_y = (height as f32 / vertical_cell_count.floor()).ceil() as usize;
        image_cell_size_x = (image_cell_size_y as f32 * cell_size_x as f32 / cell_size_y as f32).ceil() as usize;
    }
    let mut image_cells = Vec::with_capacity(region_cells.len());
    {
        let (left, top, right, bottom) = (0, 0, width, height);
        let mut y0 = top;
        loop {
            let y1 = (y0 / image_cell_size_y + 1) * image_cell_size_y;
            let y2 = min(y1, bottom);
            let mut x0 = left;
            loop {
                let x1 = (x0 / image_cell_size_x + 1) * image_cell_size_x;
                let x2 = min(x1, right);
                image_cells.push((x0, y0, x2, y2));
                if x1 >= right {
                    break;
                }
                x0 = x2;
            }
            if y1 >= bottom {
                break;
            }
            y0 = y2;
        }
    }

    // 打乱方格
    shuffle(&mut image_cells, seed);

    // 这里应该是 image_cells.len() <= region_cells.len()
    // 将 cells 光栅化成 UV 映射贴图
    let mut texture_rgba32f = vec![(0f32, 0f32, 0f32, 0f32); width * height];
    for i in 0..min(image_cells.len(), region_cells.len()) {
        let (region_cell_left, region_cell_top, region_cell_right, region_cell_bottom) = region_cells[i];
        let (image_cell_left, image_cell_top, image_cell_right, image_cell_bottom) = image_cells[i];
        let current_left = (image_cell_left as f32 + (region_cell_left as f32 - image_cell_left as f32) * progress) as usize;
        let current_top = (image_cell_top as f32 + (region_cell_top as f32 - image_cell_top as f32) * progress) as usize;
        let current_right = (image_cell_right as f32 + (region_cell_right as f32 - image_cell_right as f32) * progress) as usize;
        let current_bottom = (image_cell_bottom as f32 + (region_cell_bottom as f32 - image_cell_bottom as f32) * progress) as usize;
        for y in current_top..current_bottom {
            for x in current_left..current_right {
                let original_x = image_cell_left as f32 + (x as f32 - current_left as f32) / (current_right as f32 - current_left as f32) * (image_cell_right as f32 - image_cell_left as f32);
                let original_y = image_cell_top as f32 + (y as f32 - current_top as f32) / (current_bottom as f32 - current_top as f32) * (image_cell_bottom as f32 - image_cell_top as f32);
                texture_rgba32f[y * width + x] = (
                    original_x / width as f32,
                    original_y / height as f32,
                    0.0,
                    1.0,
                );
            }
        }
    }
    texture_rgba32f
}
