// ==UserScript==
// @name         obs-video-tile-remap 解码
// @namespace    http://tampermonkey.net/
// @version      0.1
// @description  try to take over the world!
// @author       Ganlv
// @homepage     https://github.com/ganlvtech/obs-video-tile-remap
// @match        https://live.bilibili.com/*
// @icon         https://live.bilibili.com/favicon.ico
// @grant        GM_registerMenuCommand
// @grant        GM_unregisterMenuCommand
// @grant        GM_getValue
// @grant        GM_setValue
// ==/UserScript==

(function () {
    'use strict';

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

    /**
     * 标准伪随机数算法
     */
    class PRNG {
        state;

        constructor(seed) {
            this.state = BigInt(seed);
        }

        /**
         * 获取随机数序列中的下一随机数
         * @returns {number}
         */
        nextInt() {
            this.state = (((this.state * 1103515245n) & 0xffffffffn) + 12345n) & 0xffffffffn;
            return Number(this.state & 0x7fffffffn);
        }
    }

    /**
     * 标准 Hash 算法
     * @param {string} s
     * @returns {number}
     */
    function hashcode(s) {
        const encoder = new TextEncoder();
        const view = encoder.encode(s);
        let h = 0n;
        for (const x of view) {
            h = (h * 31n + BigInt(x)) & 0xffffffffn;
        }
        return Number(h);
    }

    /**
     * 字符串转种子
     * @param {string} s
     * @returns {number}
     */
    function string_to_seed(s) {
        if (/^\d{1,10}$/.test(s)) {
            const n = parseInt(s);
            if (n > 0xffffffff) {
                return hashcode(s);
            }
            return n;
        } else {
            return hashcode(s);
        }
    }

    /**
     * 标准洗牌算法
     * @param {any[]} list
     * @param {number} seed
     */
    function shuffle(list, seed) {
        const prng = new PRNG(seed);
        const len = list.length;
        for (let i = 0; i < len; i++) {
            const r = prng.nextInt();
            const j = i + r % (len - i);
            const temp = list[j];
            list[j] = list[i];
            list[i] = temp;
        }
    }

    /**
     * 生成反向 UV 映射贴图
     * @param {number} seed
     * @param {number} width
     * @param {number} height
     * @param {number} cell_size_x
     * @param {number} cell_size_y
     * @param {[number, number, number, number][]} regions
     * @returns {Float32Array}
     */
    function generate_reverse_uv_map_texture(seed, width, height, cell_size_x, cell_size_y, regions) {
        // 将编码区域划分网格
        const region_cells = [];
        for (const region of regions) {
            const [left, top, right, bottom] = region;
            let y0 = top;
            while (true) {
                let y1 = (Math.floor(y0 / cell_size_y) + 1) * cell_size_y;
                let y2 = Math.min(y1, bottom);
                let x0 = left;
                while (true) {
                    let x1 = (Math.floor(x0 / cell_size_x) + 1) * cell_size_x;
                    let x2 = Math.min(x1, right);
                    region_cells.push([x0, y0, x2, y2]);
                    if (x1 >= right) {
                        break;
                    }
                    x0 = x2;
                }
                if (y1 >= bottom) {
                    break;
                }
                y0 = y2;
            }
        }

        // 将原始视频区域划分网格
        const vertical_cell_count = Math.sqrt(region_cells.length * height / width * cell_size_x / cell_size_y);
        let image_cell_size_y = Math.ceil(height / vertical_cell_count);
        let image_cell_size_x = Math.ceil(image_cell_size_y * cell_size_x / cell_size_y);
        // 如果格子不够的话，那么需要将调整格子大小，并浪费一些格子
        if (Math.floor((width + image_cell_size_x - 1) / image_cell_size_x) * Math.floor((height + image_cell_size_y - 1) / image_cell_size_y) > region_cells.length) {
            image_cell_size_y = Math.ceil(height / Math.floor(vertical_cell_count));
            image_cell_size_x = Math.ceil(image_cell_size_y * cell_size_x / cell_size_y);
        }
        const image_cells = [];
        {
            const left = 0;
            const top = 0;
            const right = width;
            const bottom = height;
            let y0 = top;
            while (true) {
                const y1 = (Math.floor(y0 / image_cell_size_y) + 1) * image_cell_size_y;
                const y2 = Math.min(y1, bottom);
                let x0 = left;
                while (true) {
                    let x1 = (Math.floor(x0 / image_cell_size_x) + 1) * image_cell_size_x;
                    let x2 = Math.min(x1, right);
                    image_cells.push([x0, y0, x2, y2]);
                    if (x1 >= right) {
                        break;
                    }
                    x0 = x2;
                }
                if (y1 >= bottom) {
                    break;
                }
                y0 = y2;
            }
        }

        // 打乱方格
        shuffle(image_cells, seed);

        // 这里应该是 image_cells.len() <= region_cells.len()
        // 将 cells 光栅化成 UV 映射贴图
        const texture_rgba32f = new Float32Array(width * height * 4);
        const len = Math.min(image_cells.length, region_cells.length);
        for (let i = 0; i < len; i++) {
            const [region_cell_left, region_cell_top, region_cell_right, region_cell_bottom] = region_cells[i];
            const [image_cell_left, image_cell_top, image_cell_right, image_cell_bottom] = image_cells[i];
            for (let y = image_cell_top; y < image_cell_bottom; y++) {
                for (let x = image_cell_left; x < image_cell_right; x++) {
                    const original_x = region_cell_left + (x - image_cell_left) / (image_cell_right - image_cell_left) * (region_cell_right - region_cell_left);
                    const original_y = region_cell_top + (y - image_cell_top) / (image_cell_bottom - image_cell_top) * (region_cell_bottom - region_cell_top);
                    const base_index = 4 * (y * width + x);
                    texture_rgba32f[base_index] = original_x / width;
                    texture_rgba32f[base_index + 1] = original_y / height;
                    texture_rgba32f[base_index + 2] = 0.0;
                    texture_rgba32f[base_index + 3] = 1.0;
                }
            }
        }
        return texture_rgba32f;
    }

    /**
     * 开始反向复原视频
     *
     * @param {string} seed_string
     * @param {number} width
     * @param {number} height
     * @param {number} cell_size_x
     * @param {number} cell_size_y
     * @param {[number, number, number, number][]} regions
     */
    function run(seed_string, width, height, cell_size_x, cell_size_y, regions) {
        const video = document.querySelector('video');

        // 创建 canvas 元素
        const canvas = document.createElement('canvas');
        canvas.width = video.videoWidth;
        canvas.height = video.videoHeight;
        canvas.style.pointerEvents = 'none';
        canvas.style.objectFit = 'contain';
        const onresize = () => {
            canvas.style.position = video.style.position;
            canvas.style.top = video.style.top;
            canvas.style.left = video.style.left;
            canvas.style.zIndex = String(parseInt(video.style.zIndex) + 1);
            canvas.style.width = video.style.width;
            canvas.style.height = video.style.height;
        }
        onresize();
        setInterval(onresize, 1000);
        video.insertAdjacentElement('afterend', canvas);
        const gl = canvas.getContext('webgl2');

        // 编译 shader
        const vertex_shader = gl.createShader(gl.VERTEX_SHADER);
        const fragment_shader = gl.createShader(gl.FRAGMENT_SHADER);
        gl.shaderSource(vertex_shader, `
attribute vec4 aVertexPosition;
attribute vec2 aTextureCoord;
varying highp vec2 vTextureCoord;
void main() {
  gl_Position = aVertexPosition;
  vTextureCoord = aTextureCoord;
}`);
        gl.shaderSource(fragment_shader, `
varying highp vec2 vTextureCoord;
uniform sampler2D uSamplerVideo;
uniform sampler2D uSamplerUvMap;
void main(void) {
  gl_FragColor = texture2D(uSamplerVideo, texture2D(uSamplerUvMap, vTextureCoord).xy);
}`);
        gl.compileShader(vertex_shader);
        gl.compileShader(fragment_shader);
        const shader_program = gl.createProgram();
        gl.attachShader(shader_program, vertex_shader);
        gl.attachShader(shader_program, fragment_shader);
        gl.linkProgram(shader_program);
        if (!gl.getProgramParameter(shader_program, gl.LINK_STATUS)) {
            throw new Error(`Unable to initialize the shader program: ${gl.getProgramInfoLog(shader_program)}`);
        }
        gl.useProgram(shader_program);

        // 清空场景
        gl.clearColor(0.0, 0.0, 0.0, 0.0);
        gl.clear(gl.COLOR_BUFFER_BIT);

        // 准备顶点坐标数据
        const vertex_position_left = -1;
        const vertex_position_top = 1;
        const vertex_position_right = 1;
        const vertex_position_bottom = -1;
        const vertex_position_list = [
            vertex_position_right, vertex_position_top, // 右上
            vertex_position_left, vertex_position_top, // 左上
            vertex_position_right, vertex_position_bottom, // 右下
            vertex_position_left, vertex_position_bottom, // 左下
        ];
        const position_buffer = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, position_buffer);
        gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(vertex_position_list), gl.STATIC_DRAW); // OpenGL 的坐标是右手系，左下角是 -1 -1，右上角是 1 1
        gl.vertexAttribPointer(gl.getAttribLocation(shader_program, 'aVertexPosition'), 2, gl.FLOAT, false, 0, 0);
        gl.enableVertexAttribArray(gl.getAttribLocation(shader_program, 'aVertexPosition'));

        // 准备顶点 UV 数据
        const texture_coord_buffer = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, texture_coord_buffer);
        gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([1, 0, 0, 0, 1, 1, 0, 1]), gl.STATIC_DRAW); // OpenGL 的贴图左上角是 0 0，右下角是 1 1
        gl.vertexAttribPointer(gl.getAttribLocation(shader_program, 'aTextureCoord'), 2, gl.FLOAT, false, 0, 0);
        gl.enableVertexAttribArray(gl.getAttribLocation(shader_program, 'aTextureCoord'));

        // 准备视频贴图
        gl.activeTexture(gl.TEXTURE0);
        const video_texture = gl.createTexture();
        gl.bindTexture(gl.TEXTURE_2D, video_texture);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, video);
        gl.uniform1i(gl.getUniformLocation(shader_program, 'uSamplerVideo'), 0);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

        // 准备 UV 映射贴图
        gl.activeTexture(gl.TEXTURE1);
        const uv_map_texture = gl.createTexture();
        gl.bindTexture(gl.TEXTURE_2D, uv_map_texture);
        if (!regions || regions.length === 0) {
            regions = [[0, 0, width, height]];
        }
        const uv_map_texture_buffer = generate_reverse_uv_map_texture(string_to_seed(seed_string), width, height, cell_size_x, cell_size_y, regions);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA32F, width, height, 0, gl.RGBA, gl.FLOAT, uv_map_texture_buffer);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST); // 尺寸非 2 的幂的贴图，只能使用 NEAREST
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST); // 尺寸非 2 的幂的贴图，只能使用 NEAREST
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); // 尺寸非 2 的幂的贴图，只能使用 CLAMP_TO_EDGE
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE); // 尺寸非 2 的幂的贴图，只能使用 CLAMP_TO_EDGE
        gl.uniform1i(gl.getUniformLocation(shader_program, 'uSamplerUvMap'), 1);

        // 绘制场景
        const update = () => {
            gl.activeTexture(gl.TEXTURE0);
            gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, video);
            gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
            requestAnimationFrame(update);
        };
        requestAnimationFrame(update);
    }

    GM_registerMenuCommand('使用默认参数解码视频', () => {
        run('0', 1920, 1080, 16, 16, [[0, 0, 1920, 1080]]);
    });
    GM_registerMenuCommand('自定义参数解码视频', () => {
        const config = window.prompt('seed_string,width,height,cell_size_x,cell_size_y,regions', GM_getValue('config', '0,1920,1080,16,16,[[0,0,1920,1080]]'));
        if (config) {
            GM_setValue('config', config);
            let seed_string = '';
            let width = 1920;
            let height = 1080;
            let cell_size_x = 16;
            let cell_size_y = 16;
            let regions = undefined;
            const matches = config.match(/^(.+?),(\d+),(\d+),(\d+),(\d+),(\[.*\])$/);
            if (matches) {
                // 最复杂的格式
                seed_string = matches[1];
                width = parseInt(matches[2]);
                height = parseInt(matches[3]);
                cell_size_x = parseInt(matches[4]);
                cell_size_y = parseInt(matches[5]);
                regions = JSON.parse(matches[6]);
            } else {
                const matches = config.match(/^(.+?),(\d+),(\d+),(\d+),(\d+)$/);
                if (matches) {
                    // 不需要编码区域的格式
                    seed_string = matches[1];
                    width = parseInt(matches[2]);
                    height = parseInt(matches[3]);
                    cell_size_x = parseInt(matches[4]);
                    cell_size_y = parseInt(matches[5]);
                } else {
                    // 如果不包含英文逗号，则直接认为是密码
                    if (!config.includes(',')) {
                        seed_string = config;
                    }
                }
            }
            if (seed_string !== '') {
                run(seed_string, width, height, cell_size_x, cell_size_y, regions);
            }
        }
    });
})();
