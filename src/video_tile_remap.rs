use std::ffi::{CStr, CString};
use std::mem::size_of;
use std::ptr::{null, null_mut};
use bindings::{blog, gs_color_format_GS_RGBA, gs_color_format_GS_RGBA32F, gs_effect_create, gs_effect_destroy, gs_effect_get_param_by_name, gs_effect_set_texture, gs_effect_t, gs_texture_create, gs_texture_destroy, gs_texture_t, LOG_WARNING, obs_allow_direct_render_OBS_ALLOW_DIRECT_RENDERING, obs_data_get_double, obs_data_get_int, obs_data_get_string, obs_data_set_default_double, obs_data_set_default_int, obs_data_set_default_string, obs_data_t, obs_enter_graphics, obs_leave_graphics, obs_properties_add_float_slider, obs_properties_add_int, obs_properties_add_text, obs_properties_create, obs_properties_t, obs_register_source_s, obs_source_info, obs_source_process_filter_begin, obs_source_process_filter_end, obs_source_t, obs_source_type_OBS_SOURCE_TYPE_FILTER, OBS_SOURCE_VIDEO, obs_text_type_OBS_TEXT_DEFAULT, obs_text_type_OBS_TEXT_INFO};
use uv_map::{generate_uv_map_texture, parse_regions, string_to_seed};

const EFFECT_CONTENT: &[u8] = include_bytes!("video_tile_remap.effect");

pub unsafe fn register() {
    obs_register_source_s(&obs_source_info {
        id: "obs-video-tile-remap\0".as_ptr().cast(),
        type_: obs_source_type_OBS_SOURCE_TYPE_FILTER,
        output_flags: OBS_SOURCE_VIDEO,
        get_name: Some(filter_get_name),
        create: Some(filter_create),
        update: Some(filter_update),
        destroy: Some(filter_destroy),
        get_defaults: Some(get_defaults),
        get_properties: Some(get_properties),
        video_render: Some(video_render),
        ..Default::default()
    }, size_of::<obs_source_info>());
}

struct VideoCellReorder {
    source: *mut obs_source_t,
    effect: *mut gs_effect_t,
    texture_data: Vec<(f32, f32, f32, f32)>,
    texture: *mut gs_texture_t,
}

unsafe extern "C" fn filter_get_name(_type_data: *mut ::std::os::raw::c_void) -> *const ::std::os::raw::c_char {
    "Video Tile Remap\0".as_ptr().cast()
}

unsafe extern "C" fn get_properties(_data: *mut ::std::os::raw::c_void) -> *mut obs_properties_t {
    let props = obs_properties_create();
    let _ = obs_properties_add_text(props, "seed\0".as_ptr().cast(), "随机数种子\0".as_ptr().cast(), obs_text_type_OBS_TEXT_DEFAULT);
    let _ = obs_properties_add_int(props, "width\0".as_ptr().cast(), "宽度，推荐为 1920\0".as_ptr().cast(), 1, 3840, 1);
    let _ = obs_properties_add_int(props, "height\0".as_ptr().cast(), "高度，推荐为 1080\0".as_ptr().cast(), 1, 2160, 1);
    let _ = obs_properties_add_int(props, "cell_size_x\0".as_ptr().cast(), "方格宽度，推荐为 16\0".as_ptr().cast(), 1, 2048, 1);
    let _ = obs_properties_add_int(props, "cell_size_y\0".as_ptr().cast(), "方格高度，推荐为 16\0".as_ptr().cast(), 1, 2048, 1);
    let _ = obs_properties_add_text(props, "regions\0".as_ptr().cast(), "编码区域，可以留空\0".as_ptr().cast(), obs_text_type_OBS_TEXT_DEFAULT);
    let _ = obs_properties_add_float_slider(props, "progress\0".as_ptr().cast(), "变化进度，通常为 1.000\0".as_ptr().cast(), 0.0, 1.0, 0.001);
    let _ = obs_properties_add_text(props, "help_1\0".as_ptr().cast(), "说明：数字随机数种子应该在 0 ~ 4294967295 之间。不在这个区间的数字或者非数字会被自动使用哈希算法转换成数字。\0".as_ptr().cast(), obs_text_type_OBS_TEXT_INFO);
    let _ = obs_properties_add_text(props, "help_2\0".as_ptr().cast(), "说明：编码区域的格式为[左,上,右,下]，必须使用英文逗号分割，支持填写多个区域，例如 [0,0,1920,100],[0,100,200,800],[1600,100,1920,800],[0,800,1920,1080]，编码区域的左上右下推荐都对齐 16 的整数倍。\0".as_ptr().cast(), obs_text_type_OBS_TEXT_INFO);
    let _ = obs_properties_add_text(props, "LICENSE\0".as_ptr().cast(), "本插件基于 GPLv2 开源。你可以在 https://github.com/ganlvtech/obs-video-tile-remap 免费下载。\0".as_ptr().cast(), obs_text_type_OBS_TEXT_INFO);
    props
}

unsafe extern "C" fn get_defaults(settings: *mut obs_data_t) {
    obs_data_set_default_string(settings, "seed\0".as_ptr().cast(), "0\0".as_ptr().cast());
    obs_data_set_default_int(settings, "width\0".as_ptr().cast(), 1920);
    obs_data_set_default_int(settings, "height\0".as_ptr().cast(), 1080);
    obs_data_set_default_int(settings, "cell_size_x\0".as_ptr().cast(), 16);
    obs_data_set_default_int(settings, "cell_size_y\0".as_ptr().cast(), 16);
    obs_data_set_default_string(settings, "regions\0".as_ptr().cast(), "[0,0,1920,1080]\0".as_ptr().cast());
    obs_data_set_default_double(settings, "progress\0".as_ptr().cast(), 1.0);
}

unsafe extern "C" fn filter_create(settings: *mut obs_data_t, source: *mut obs_source_t) -> *mut ::std::os::raw::c_void {
    obs_enter_graphics();
    let uv_mapping_effect_content_cstring = CString::new(EFFECT_CONTENT).unwrap();
    let effect = gs_effect_create(uv_mapping_effect_content_cstring.as_ptr(), null(), null_mut());
    obs_leave_graphics();

    let filter = Box::into_raw(Box::new(VideoCellReorder {
        source,
        effect,
        texture_data: vec![],
        texture: null_mut() as _,
    })).cast();

    filter_update(filter, settings);
    filter
}

unsafe extern "C" fn filter_update(data: *mut ::std::os::raw::c_void, settings: *mut obs_data_t) {
    let filter = &mut *(data as *mut VideoCellReorder);
    let seed = string_to_seed(CStr::from_ptr(obs_data_get_string(settings, "seed\0".as_ptr().cast())).to_bytes());
    let width = obs_data_get_int(settings, "width\0".as_ptr().cast()) as usize;
    let height = obs_data_get_int(settings, "height\0".as_ptr().cast()) as usize;
    let cell_size_x = obs_data_get_int(settings, "cell_size_x\0".as_ptr().cast()) as usize;
    let cell_size_y = obs_data_get_int(settings, "cell_size_y\0".as_ptr().cast()) as usize;
    let regions = match parse_regions(CStr::from_ptr(obs_data_get_string(settings, "regions\0".as_ptr().cast())).to_string_lossy().as_ref()) {
        Ok(regions) => {
            if regions.len() == 0 {
                blog(LOG_WARNING, format!("parse_regions result len == 0\0").as_ptr().cast());
                vec![(0, 0, width, height)]
            } else {
                regions
            }
        }
        Err(e) => {
            blog(LOG_WARNING, format!("parse_regions error: {}\0", e).as_ptr().cast());
            vec![(0, 0, width, height)]
        }
    };
    let progress = obs_data_get_double(settings, "progress\0".as_ptr().cast()) as f32;
    obs_enter_graphics();
    if !filter.texture.is_null() {
        gs_texture_destroy(filter.texture);
    }
    let texture_data = generate_uv_map_texture(seed, width, height, cell_size_x, cell_size_y, &regions, progress);
    let texture = gs_texture_create(width as _, height as _, gs_color_format_GS_RGBA32F, 1, &mut (texture_data.as_ptr() as *const u8), 0);
    filter.texture_data = texture_data;
    filter.texture = texture;
    obs_leave_graphics();
}

unsafe extern "C" fn video_render(data: *mut ::std::os::raw::c_void, _effect: *mut gs_effect_t) {
    let filter = &mut *(data as *mut VideoCellReorder);
    if !filter.texture.is_null() {
        if !obs_source_process_filter_begin(filter.source, gs_color_format_GS_RGBA, obs_allow_direct_render_OBS_ALLOW_DIRECT_RENDERING) {
            return;
        }
        gs_effect_set_texture(gs_effect_get_param_by_name(filter.effect, "mapperImage\0".as_ptr().cast()), filter.texture);
        obs_source_process_filter_end(filter.source, filter.effect, 0, 0);
    }
}

unsafe extern "C" fn filter_destroy(data: *mut ::std::os::raw::c_void) {
    let filter = data as *mut VideoCellReorder;
    if !filter.is_null() {
        obs_enter_graphics();
        if !(*filter).texture.is_null() {
            gs_texture_destroy((*filter).texture);
        }
        gs_effect_destroy((*filter).effect);
        obs_leave_graphics();
        let _ = Box::from_raw(filter);
    }
}
