extern crate pathdiff;
use application::{App, AssetType, Layer, Marker, PropertySelect, Tile, TileSheet};
use glam::{Mat4, Vec3, Vec2, Quat};
use imgui::{DragDropFlags, FontConfig, Selectable, TextureId};
use renderer::{Line, ShaderProgram, Sprite};
use std::{time::Instant, collections::HashMap};
use glow::HasContext;
use glutin::{event_loop::EventLoop, WindowedContext, dpi, event::{ElementState, KeyboardInput, VirtualKeyCode}};
use imgui_winit_support::WinitPlatform;

mod renderer;
mod application;

const TITLE: &str = "Lilah Editor";

type Window = WindowedContext<glutin::PossiblyCurrent>;

fn window_size(window: Window, winit_platform: WinitPlatform) -> Vec2 {
    Vec2::new(
        window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).width,
        window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).height
    )
}

fn main() {
    let mut app = App::new();

    let (event_loop, window) = create_window();
    let (mut winit_platform, mut imgui_context) = imgui_init(&window);

    let gl = glow_context(&window);

    let mut ig_renderer = imgui_glow_renderer::AutoRenderer::initialize(
        gl, 
        &mut imgui_context
    ).expect("failed to create renderer"); 

    unsafe {
        *crate::renderer::PROJECTION_MATRIX = 
            Mat4::orthographic_rh_gl(
            -window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).width/2.0, 
            window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).width/2.0, 
            -window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).height/2.0,
            window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).height/2.0, 
            1000.0, 
            -1000.0
            );
        *crate::renderer::VIEW_MATRIX = 
            Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0));
    }

    unsafe {
        ig_renderer.gl_context().enable(glow::BLEND);
        ig_renderer.gl_context().blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
    }

    let mut last_frame = Instant::now();

    let program = unsafe { 
        let shaders = [
            renderer::Shader::new(
                ig_renderer.gl_context(), 
                crate::renderer::DEFAULT_VERT, 
                glow::VERTEX_SHADER
            ).unwrap(), 
            renderer::Shader::new(
                ig_renderer.gl_context(), 
                crate::renderer::DEFAULT_FRAG, 
                glow::FRAGMENT_SHADER
            ).unwrap() 
        ];

        ShaderProgram::new(ig_renderer.gl_context(), &shaders).unwrap()
    };

    let line_program = unsafe { 
        let shaders = [
            renderer::Shader::new(
                ig_renderer.gl_context(), 
                crate::renderer::LINE_VERT, 
                glow::VERTEX_SHADER
            ).unwrap(), 
            renderer::Shader::new(
                ig_renderer.gl_context(), 
                crate::renderer::Line_FRAG, 
                glow::FRAGMENT_SHADER
            ).unwrap() 
        ];

        ShaderProgram::new(ig_renderer.gl_context(), &shaders).unwrap()
    };

    let mut camera = Vec2::new(0.0, 0.0);
    let mut camera_zoom = 1.0;
    let mut last_click = Vec2::new(0.0, 0.0);
    let mut tile_count = [0, 0];
    let mut win_size = [800f32, 600f32];
    let mut current_tile = (0u32, 0u32);
    let mut property_select = PropertySelect::None;
    let mut marker_spr = Sprite::new("lilah__editor__internal__ignore__marker_icon.png");
    app.load_texture_internal(ig_renderer.gl_context(), "marker_icon.png");
    marker_spr.load(ig_renderer.gl_context(), &program, &app.textures);

    let yellow = [1.0, 0.886, 0.482,1.0];
    let light_yellow = [1.0, 0.933, 0.698,1.0];
    let dark_yellow = [0.478, 0.455, 0.361,1.0];
    let light_dark_yellow = [0.639, 0.608, 0.486, 1.0];
    
    imgui_context.style_mut().colors[imgui::StyleColor::Button as usize] = dark_yellow;
    imgui_context.style_mut().colors[imgui::StyleColor::ButtonHovered as usize] = light_dark_yellow;
    imgui_context.style_mut().colors[imgui::StyleColor::ButtonActive as usize] = [0.78, 0.682, 0.333, 1.0];

    imgui_context.style_mut().colors[imgui::StyleColor::Tab as usize] = dark_yellow;
    imgui_context.style_mut().colors[imgui::StyleColor::TabHovered as usize] = [0.78, 0.682, 0.333, 1.0];
    imgui_context.style_mut().colors[imgui::StyleColor::TabActive as usize] = [0.72, 0.632, 0.28, 1.0];

    imgui_context.style_mut().colors[imgui::StyleColor::ChildBg as usize] = [0.259, 0.255, 0.239, 0.5];
    imgui_context.style_mut().colors[imgui::StyleColor::FrameBgActive as usize] = light_dark_yellow;
    imgui_context.style_mut().colors[imgui::StyleColor::FrameBgHovered as usize] = light_dark_yellow;
    imgui_context.style_mut().colors[imgui::StyleColor::FrameBg as usize] = [0.259, 0.255, 0.2393, 0.5];

    imgui_context.style_mut().colors[imgui::StyleColor::CheckMark as usize] = [1.0,1.0,1.0,1.0];

    imgui_context.style_mut().colors[imgui::StyleColor::Text as usize] = [0.1,0.1,0.1,1.0];

    imgui_context.style_mut().colors[imgui::StyleColor::MenuBarBg as usize] = yellow;
    imgui_context.style_mut().colors[imgui::StyleColor::HeaderHovered as usize] = light_yellow;
    imgui_context.style_mut().colors[imgui::StyleColor::HeaderActive as usize] = light_yellow;

    imgui_context.style_mut().colors[imgui::StyleColor::TextSelectedBg as usize] = dark_yellow;

    imgui_context.style_mut().colors[imgui::StyleColor::PopupBg as usize] = [1.0,1.0,1.0,1.0];

    imgui_context.style_mut().colors[imgui::StyleColor::WindowBg as usize] = [0.129, 0.129, 0.125, 0.9];
    imgui_context.style_mut().colors[imgui::StyleColor::TitleBg as usize] = light_yellow;
    imgui_context.style_mut().colors[imgui::StyleColor::TitleBgActive as usize] = light_yellow;


    event_loop.run(move |event, _, control_flow| {
        match event {
            glutin::event::Event::NewEvents(_) => {
                let now = Instant::now();
                imgui_context
                    .io_mut()
                    .update_delta_time(now.duration_since(last_frame));
                last_frame = now;
            }
            glutin::event::Event::MainEventsCleared => {
                winit_platform
                    .prepare_frame(imgui_context.io_mut(), window.window())
                    .unwrap();
                window.window().request_redraw();
            }
            glutin::event::Event::RedrawRequested(_) => {
                let ui = imgui_context.frame();
                let window_size = (
                    window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).width,
                    window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).height
                );

                if ui.is_key_down(imgui::Key::Space) {
                    let drag = ui.mouse_drag_delta_with_button(imgui::MouseButton::Left);
                    if Vec2::new(-drag[0], drag[1]).length() > 0.5f32 {
                        camera += Vec2::new(-drag[0], drag[1])*0.05;
                        unsafe {
                            *crate::renderer::VIEW_MATRIX = 
                            Mat4::from_translation(Vec3::new(-camera.x, -camera.y, 0.0));
                        }
                    }
                }

                // let prev_zoom = camera_zoom;
                // camera_zoom += ui.io().mouse_wheel*0.05;
                // if prev_zoom != camera_zoom {
                //     unsafe {
                //         *crate::renderer::VIEW_MATRIX = 
                //         Mat4::from_scale_rotation_translation(
                //             Vec3::ONE*camera_zoom,
                //             Quat::from_rotation_z(0.0),
                //             Vec3::new(-camera.x, -camera.y, 0.0)
                //         );
                //     }
                // }

                if let PropertySelect::Marker(marker) = property_select {
                    if let Some(scene) = app.current_scene.as_mut() {  
                        if let Some(marker) = scene.markers.get_mut(marker) {
                            if ui.is_mouse_clicked(imgui::MouseButton::Left) { 
                                let mouse_pos = Vec2::from_slice(&ui.io().mouse_pos);

                                let model = 
                                Mat4::IDENTITY * 
                                Mat4::from_scale_rotation_translation( 
                                    Vec3::new(1.0, 1.0, 1.0),
                                    Quat::from_rotation_z(0.0), 
                                    Vec3::new(mouse_pos.x-(window_size.0/2.0), (window_size.1/2.0)-mouse_pos.y, 0.0)
                                );

                                let view = unsafe { *crate::renderer::VIEW_MATRIX };
                                let projection = unsafe { *crate::renderer::PROJECTION_MATRIX };

                                let mvp =  model * view.inverse() * projection;
                                let position_v3 = mvp.to_scale_rotation_translation().2;
                                let position = Vec2::new(position_v3.x, position_v3.y);

                                
                                if application::aabb(
                                    (position)-Vec2::new(3.0,-3.0), 
                                    Vec2::new(6.0,6.0), 
                                    Vec2::new(marker.position[0]-3.0, marker.position[1]+50.0), 
                                    Vec2::new(6.0, 50.0)
                                ) {
                                    last_click = position;
                                }

                                if application::aabb(
                                    (position)-Vec2::new(3.0,-3.0), 
                                    Vec2::new(6.0,6.0), 
                                    Vec2::new(marker.position[0]+14.0, marker.position[1]+3.0), 
                                    Vec2::new(50.0, 6.0)
                                ) {
                                    last_click = position;
                                }
                            }

                            if ui.is_mouse_dragging(imgui::MouseButton::Left) {
                                let mouse_pos = Vec2::from_slice(&ui.io().mouse_pos);

                                let model = 
                                Mat4::IDENTITY * 
                                Mat4::from_scale_rotation_translation( 
                                    Vec3::new(1.0, 1.0, 1.0),
                                    Quat::from_rotation_z(0.0), 
                                    Vec3::new(mouse_pos.x-(window_size.0/2.0), (window_size.1/2.0)-mouse_pos.y, 0.0)
                                );

                                let view = unsafe { *crate::renderer::VIEW_MATRIX };
                                let projection = unsafe { *crate::renderer::PROJECTION_MATRIX };

                                let mvp =  model * view.inverse() * projection;
                                let position_v3 = mvp.to_scale_rotation_translation().2;
                                let position = Vec2::new(position_v3.x, position_v3.y);

                                if application::aabb(
                                    (position)-Vec2::new(10.0,-10.0), 
                                    Vec2::new(20.0,20.0), 
                                    Vec2::new(marker.position[0]-10.0, marker.position[1]+10.0), 
                                    Vec2::new(20.0, 20.0)
                                ) {
                                    marker.position[0] = position.x;
                                    marker.position[1] = position.y;
                                }
                            }
                        }
                    }
                }

                unsafe { ig_renderer.gl_context().clear(glow::COLOR_BUFFER_BIT) };

                for sprs in &app.sprite_buffer {
                    for spr in sprs {
                        spr.1.draw(ig_renderer.gl_context(), &program, &app.textures);
                    }
                }

                if let Some(scene) = app.current_scene.as_ref() {
                    for marker in scene.markers.iter().enumerate() {
                        marker_spr.position = Vec2::new(marker.1.position[0], marker.1.position[1]+30.0);
                        marker_spr.draw(ig_renderer.gl_context(), &program, &app.textures);
                        if let PropertySelect::Marker(m) = property_select {
                            if m == marker.0 {
                                Line::draw(
                                    ig_renderer.gl_context(), 
                                    &line_program, 
                                    Vec2::new(marker.1.position[0], marker.1.position[1]),
                                    Vec2::new(marker.1.position[0], marker.1.position[1]+64.0),
                                    &[0.0,1.0,0.0,1.0]
                                );

                                Line::draw(
                                    ig_renderer.gl_context(), 
                                    &line_program, 
                                    Vec2::new(marker.1.position[0], marker.1.position[1]),
                                    Vec2::new(marker.1.position[0]+64.0, marker.1.position[1]),
                                    &[1.0,0.0,0.0,1.0]
                                );
                            }
                        }
                    }

                    let sheet = scene.tile_sheets.iter()
                        .find(|&a| a.path == app.get_tile_sheet());

                    if let Some(sheet) = sheet {
                        let size = sheet.tile_size;

                        let mut offset = Vec2::new(camera.x.rem_euclid(size.0 as f32)-(size.0 as f32/2.0), camera.y.rem_euclid(size.1 as f32)-(size.1 as f32/2.0));

                        let w =  window_size.0 / size.0 as f32;
                        let h =  window_size.1 / size.1 as f32;

                        offset -= camera;

                        for i in 0..(w as i32 * 2) {
                            Line::draw(
                                ig_renderer.gl_context(), 
                                &line_program, 
                                Vec2::new( size.0 as f32 * (i-w as i32) as f32, -h * size.1 as f32) - offset,
                                Vec2::new( size.0 as f32 * (i-w as i32) as f32, window_size.1) - offset,
                                &[1.0,1.0,1.0,0.25]
                            );
                        }

                        for i in 0..(h as i32 * 2) {
                            
                            Line::draw(
                                ig_renderer.gl_context(), 
                                &line_program, 
                                Vec2::new(-w * size.0 as f32, size.1 as f32 * (i-h as i32) as f32) - offset,
                                Vec2::new(window_size.0, size.1 as f32 * (i-h as i32) as f32) - offset,
                                &[1.0,1.0,1.0,0.25]
                            );
                        }

                        Line::draw(
                            ig_renderer.gl_context(), 
                            &line_program, 
                            Vec2::new(-10.0, 0.0),
                            Vec2::new(10.0, 0.0),
                            &[1.0,1.0,1.0,1.0]
                        );
                        Line::draw(
                            ig_renderer.gl_context(), 
                            &line_program, 
                            Vec2::new(0.0, -10.0),
                            Vec2::new(0.0, 10.0),
                            &[1.0,1.0,1.0,1.0]
                        );
                    }
                }

                let mut open_window_size = false;
                if let Some(main_menu) = ui.begin_main_menu_bar() {
                    if let Some(_) = ui.begin_menu("File") {
                        if ui.menu_item("New") {
                            window.window().set_title(app.new_project());
                        }
                        if ui.menu_item("Open") {
                            window.window().set_title(app.open_project());
                            win_size = [app.config.window_size.0, app.config.window_size.1]; 
                        }
                        if app.current_project != "" {
                            if ui.menu_item("Save") {
                                app.wrangle_main();
                                app.write_config();
                            }
                        }
                    }
                    if app.current_project != "" {
                        if let Some(_) = ui.begin_menu("Project") {
                            if ui.menu_item("Run") {
                                app.run_project();
                            }
                            if let Some(_) = ui.begin_menu("Settings") {
                                if ui.menu_item("Window Size") {
                                    open_window_size = true;
                                }
                            }
                            if let Some(_) = ui.begin_menu("Assets") {
                                if let Some(_) = ui.begin_menu("Add") {
                                    if ui.menu_item("External") {
                                        app.add_external_asset();
                                    }
                                    if ui.menu_item("Embedded") {
                                        app.add_embedded_asset();
                                    }
                                }
                            }
                        }
                        if let Some(_) = ui.begin_menu("World") {
                            if ui.menu_item("New") {
                                app.new_scene();
                            }
                            if ui.menu_item("Open") {
                                app.open_scene(ig_renderer.gl_context(), &program);
                            }
                            if let Some(_) = app.current_scene.as_ref() {
                                if ui.menu_item("Save") {
                                    app.write_current_scene();
                                }
                            }
                        }
                    } 

                    if open_window_size {
                        ui.open_popup("Window Size");
                    }
                    
                    
                    let win_color = ui.push_style_color(imgui::StyleColor::PopupBg, [0.129, 0.129, 0.125, 0.9]);

                    if let Some(_) = ui.modal_popup_config("Window Size").always_auto_resize(true).begin_popup() {
                        let text_color = ui.push_style_color(imgui::StyleColor::Text, [1.0,1.0,1.0,1.0]);
                        ui.input_float2("width : height", &mut win_size).build();

                        ui.columns(2, "win_size_exit", false);
                        if ui.button("Save") {
                            app.config.window_size.0 = win_size[0];
                            app.config.window_size.1 = win_size[1];
                            ui.close_current_popup();
                        }
                        ui.next_column();
                        if ui.button("Close") {
                            win_size = [app.config.window_size.0, app.config.window_size.1]; 
                            ui.close_current_popup();
                        }
                        ui.next_column();
                        text_color.pop();
                    }

                    win_color.pop();
                                    
                    main_menu.end();
                }
                
                if app.current_project != "" {
                    ui.window("Properties")
                    .size(
                        [200.0, 
                        window_size.1-175.0-20.0], imgui::Condition::Always
                    )
                    .position(
                        [ window_size.0-200.0, 
                        20.0], 
                        imgui::Condition::Always
                    )
                    .resizable(false)
                    .collapsible(false)
                    .build(|| {
                        let mut text_color = ui.push_style_color(imgui::StyleColor::Text, [1.0,1.0,1.0,1.0]);
                        let mut hover_color = ui.push_style_color(imgui::StyleColor::HeaderHovered, [1.0,1.0,1.0,0.35]);
                        let mut active_hover_color = ui.push_style_color(imgui::StyleColor::HeaderActive, [1.0,1.0,1.0,0.5]);

                        match &property_select {
                            PropertySelect::None => {}
                            PropertySelect::Marker(marker) => {
                                if let Some(_) = ui.tab_bar("prop_main") {
                                    if let Some(scene) = app.current_scene.as_mut() {
                                        if let Some(_) = ui.tab_item("Marker") {
                                            ui.columns(1, "marker_columns", false);
                                            if let Some(m) = scene.markers.get_mut(*marker) {
                                                ui.input_text("Name", &mut m.name).build();
                                                ui.input_float2("Pos", &mut m.position).build();
                                            }
                                        }
                                    }
                                }
                            }
                            PropertySelect::Tilesheet(sheet) => {
                                if let Some(_) = ui.tab_bar("prop_main") {
                                    if let Some(_) = ui.tab_item("Tile Sheet") {
                                        if let Some(scene) = app.current_scene.as_mut() {
                                            if let Some(tilesheet) = scene.tile_sheets.get_mut(*sheet) {
                                                ui.text(format!("{}", tilesheet.filename));
                                                if ui.input_int2("Tiles", &mut tile_count).build() {
                                                    if tile_count[0] != 0 && tile_count[1] != 0 {
                                                        tilesheet.tile_size = (
                                                            tilesheet.sheet_size.0/tile_count[0] as u32, 
                                                            tilesheet.sheet_size.1/tile_count[1] as u32
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            PropertySelect::Layer => {
                                if let Some(_) = ui.tab_bar("prop_main") {
                                    if let Some(_) = ui.tab_item("Layer") {
                                        if let Some(scene) = app.current_scene.as_mut() {
                                            if let Some(layer) = scene.layers.get_mut(app.current_layer) {
                                                ui.columns(2, "Properties", true);
                                                ui.text("Tile Sheet");
                                                ui.next_column();
                                                if let Some(tilesheet) = scene.tile_sheets.get(layer.current_tile_item as usize) {
                                                    if ui.button(&tilesheet.filename) {
                                                        ui.open_popup("TileSheetPopup");                                                        
                                                    }
                                                }
                                                ui.next_column();
                                                ui.text("Tile Count");
                                                ui.next_column();
                                                ui.text(format!("{}", layer.tiles.keys().len()));
                                                ui.next_column();
                                                ui.text("Collision");
                                                ui.next_column();
                                                if layer.collision {
                                                    ui.checkbox("enabled", &mut layer.collision);
                                                } else {
                                                    ui.checkbox("disabled", &mut layer.collision);
                                                }
                                                ui.next_column();

                                                let win_color = ui.push_style_color(imgui::StyleColor::PopupBg, [0.129, 0.129, 0.125, 0.9]);
                                                if let Some(_) = ui.begin_popup("TileSheetPopup") { 
                                                    let list = scene.tile_sheets.iter().map(|TileSheet { ref filename, .. }| filename.as_str()).collect::<Vec<&str>>();
                                                    let list2 = scene.tile_sheets.iter().map(|TileSheet { ref path, .. }| path.as_str()).collect::<Vec<&str>>();
                                                    if ui.list_box("Tile Sheet", &mut layer.current_tile_item, list.as_slice(), list.len() as i32) {
                                                        layer.tile_sheet = list2[layer.current_tile_item as usize].to_string();
                                                        app.current_tile_sheet = list2[layer.current_tile_item as usize].to_string();
                                                        for j in &mut layer.tiles {
                                                            if let Some(buffer) = app.sprite_buffer.get_mut(app.current_layer) {
                                                                if let Some(spr) = buffer.get_mut(
                                                                    j.0
                                                                ) {
                                                                    spr.texture_id = list2[layer.current_tile_item as usize].to_string();
                                                                }
                                                            }
                                                            j.1.sheet = layer.tile_sheet.clone(); 
                                                        } 
                                                        ui.close_current_popup();
                                                    }
                                                }
                                                win_color.pop();
                                            }
                                        }
                                    }
                                    if let Some(_) = ui.tab_item("Tiles") {
                                        if let Some(scene) = app.current_scene.as_ref() {
                                            let sheet = scene.tile_sheets.iter()
                                                .find(|&a| a.path == app.get_tile_sheet());
                                            
                                            if let Some(sheet) = sheet {
                                                let tile_wh = sheet.get_num_of_tiles();
                                                
                                                ui.columns(tile_wh.0 as i32, "tile_cols", false);
                                                for i in 0..tile_wh.1 {
                                                    for j in 0..tile_wh.0 {
                                                        if ui.selectable(format!("{}x{}", j, i).to_string()) {
                                                            current_tile = (j as u32, i as u32);
                                                        }

                                                        ui.invisible_button("mock_btn_for_tile_img", [25.0, 25.0]);
                                                        let min = ui.item_rect_min();
                                                        let max = ui.item_rect_max();

                                                        let ratio = (
                                                            ((sheet.sheet_size.0 as f32/tile_wh.0 as f32)/sheet.sheet_size.0 as f32),
                                                            ((sheet.sheet_size.1 as f32/tile_wh.1 as f32)/sheet.sheet_size.1 as f32),
                                                        );

                                                        fn precision_f32(x: f32, decimals: u32) -> f32 {
                                                            if x == 0. || decimals == 0 {
                                                                0.
                                                            } else {
                                                                let shift = decimals as i32 - x.abs().log10().ceil() as i32;
                                                                let shift_factor = 10_f64.powi(shift) as f32;

                                                                (x * shift_factor).round() / shift_factor
                                                            }
                                                        }

                                                        let zero = (
                                                            precision_f32((j as f32) / tile_wh.0 as f32 + (1.0/sheet.sheet_size.0 as f32), 2), 
                                                            precision_f32((i as f32) / tile_wh.1 as f32 + (1.0/sheet.sheet_size.1 as f32), 2)
                                                        );

                                                        let one = (
                                                            precision_f32(zero.0+ratio.0 - (1.0/sheet.sheet_size.0 as f32) * 2.0, 2), 
                                                            precision_f32(zero.1+ratio.1 - (1.0/sheet.sheet_size.1 as f32) * 2.0, 2)
                                                        );

                                                        let new_verts =
                                                        [[zero.0, zero.1],
                                                        [one.0, zero.1],
                                                        [one.0, one.1],
                                                        [zero.0, one.1]];
                                                        let draw_list = ui.get_window_draw_list();
                                                        draw_list 
                                                        .add_image_quad(TextureId::new(u32::from(app.textures[&app.current_tile_sheet].id.0) as usize),
                                                        [min[0], min[1]], [max[0], min[1]], [max[0], max[1]], [min[0], max[1]])
                                                        .uv(new_verts[0], new_verts[1], new_verts[2], new_verts[3])
                                                        .build();

                                                        if j != tile_wh.0-1 {
                                                                ui.next_column();
                                                        }
                                                    }
                                                    ui.next_column();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            PropertySelect::Script => {
                                let mut sorted_scripts = vec!();
                                for ass in &app.config.assets {
                                    if let AssetType::Script = ass.1.type_of {
                                        sorted_scripts.push((ass.1.name.clone(), ass.1.load_order.unwrap()));
                                    }
                                }
                                sorted_scripts.sort_by(|a, b| a.1.cmp(&b.1));
                                let mut a = -1;
                                let mut b = -1;
                                for ass in sorted_scripts {
                                    ui.selectable(ass.0.clone());

                                    if let Some(tip) = ui.drag_drop_source_config("drag").begin_payload(ass.1) {
                                        ui.text(ass.0.clone());
                                        tip.end();
                                    }

                                    if let Some(target) = ui.drag_drop_target() {
                                        if let Some(Ok(payload_data)) = target
                                            .accept_payload::<usize, _>("drag", DragDropFlags::empty())
                                        {
                                            a = ass.1 as i32;
                                            b = payload_data.data as i32;
                                        }
                                        target.pop();
                                    }
                                }
                                if a != -1 && b != -1 {
                                    let mut aa_temp = None;
                                    for i in &app.config.assets {
                                        if let AssetType::Script = i.1.type_of {
                                            if let Some(lo) = i.1.load_order {
                                                if lo == a as usize {
                                                    aa_temp = Some(lo);
                                                }
                                            }
                                        }
                                    }

                                    let mut bb_temp = None;
                                    for i in &app.config.assets {
                                        if let AssetType::Script = i.1.type_of {
                                            if let Some(lo) = i.1.load_order {
                                                if lo == b as usize {
                                                    bb_temp = Some(lo);
                                                }
                                            }
                                        }
                                    }

                                    let mut bb= None;
                                    for i in &mut app.config.assets {
                                        if let AssetType::Script = i.1.type_of {
                                            if let Some(lo) = i.1.load_order {
                                                if lo == b as usize {
                                                    bb = Some(i.1);
                                                }
                                            }
                                        }
                                    }

                                    match bb {
                                        Some(bbb) => bbb.load_order = aa_temp,
                                        None => { }
                                    }

                                    let mut aa = None;
                                    for i in &mut app.config.assets {
                                        if let AssetType::Script = i.1.type_of {
                                            if let Some(lo) = i.1.load_order {
                                                if lo == a as usize {
                                                    aa = Some(i.1);
                                                }
                                            } 
                                        }
                                    }

                                    match aa {
                                        Some(aaa) => aaa.load_order = bb_temp,
                                        None => { }
                                    }
                                }
                            }
                       }
                       text_color.pop();
                       hover_color.pop();
                       active_hover_color.pop();
                    });

                    ui.window("Hierarchy")
                    .size(
                        [175.0, 
                        window_size.1-175.0-20.0], imgui::Condition::Always
                    )
                    .position(
                        [0.0, 
                        20.0], 
                        imgui::Condition::Always
                    )
                    .resizable(false)
                    .collapsible(false)
                    .build(|| {
                        let mut text_color = ui.push_style_color(imgui::StyleColor::Text, [1.0,1.0,1.0,1.0]);
                        let mut hover_color = ui.push_style_color(imgui::StyleColor::HeaderHovered, [1.0,1.0,1.0,0.35]);
                        let mut active_hover_color = ui.push_style_color(imgui::StyleColor::HeaderActive, [1.0,1.0,1.0,0.5]);

                        if let Some(_) = ui.tab_bar("main") {
                            if let Some(scene) = app.current_scene.as_mut() {
                            if let Some(_) = ui.tab_item("Layers") {
                                
                                    if ui.button("Add") {
                                        let mut new_layer = Layer::new();
                                        new_layer.tile_sheet = app.current_tile_sheet.clone();
                                        scene.layers.push(new_layer);
                                        app.sprite_buffer.push(HashMap::new());
                                    }

                                    ui.columns(3, "layers_column", false);
                                    for i in scene.layers.iter_mut().enumerate() {
                                        if i.0 == app.current_layer {
                                            ui.bullet();
                                        }
                                        ui.next_column();
                                        if ui.selectable(format!("Layer {}", i.0)) {
                                            property_select = PropertySelect::Layer;
                                            app.current_layer = i.0;
                                            app.current_tile_sheet = i.1.tile_sheet.clone();
                                            for j in &mut i.1.tiles {
                                                if let Some(buffer) = app.sprite_buffer.get_mut(i.0) {
                                                    if let Some(spr) = buffer.get_mut(
                                                        j.0
                                                    ) {
                                                        spr.texture_id = i.1.tile_sheet.clone();
                                                    }
                                                }
                                                j.1.sheet = app.current_tile_sheet.clone();
                                            } 
                                        }
                                        ui.next_column();
                                        let button_label = if i.1.visible {
                                            format!("Hide##{}", i.0)
                                        }
                                        else {
                                            format!("Show##{}", i.0)
                                        };

                                        if ui.button(button_label) {
                                            i.1.visible = !i.1.visible;

                                            for j in &mut i.1.tiles {
                                                if let Some(buffer) = app.sprite_buffer.get_mut(i.0) {
                                                    if let Some(spr) = buffer.get_mut(
                                                        j.0
                                                    ) {
                                                        spr.visible = i.1.visible;
                                                    }
                                                }
                                            }   
                                        }
                                        ui.next_column();
                                    }
                                    ui.columns(1, "layers_column_2", false);
                                }
                                if let Some(_) = ui.tab_item("Markers") {
                                    if let Some(scene) = app.current_scene.as_mut() {
                                        if ui.button("Add") {
                                            scene.markers.push(Marker { position: [0.0, 0.0], name: format!("Marker {}", scene.markers.len()).to_string() });
                                        }

                                        ui.columns(2, "markers_columns", false);
                                        let mut for_deletion = vec!();
                                        for marker in scene.markers.iter().enumerate() {
                                            if ui.selectable(marker.1.name.clone()) {
                                                property_select = PropertySelect::Marker(marker.0);
                                            }
                                            ui.next_column();
                                            if ui.button(format!("Delete##{}", marker.0)) {
                                                for_deletion.push(marker.0);
                                            }
                                            ui.next_column();
                                        }

                                        for i in for_deletion {
                                            scene.markers.remove(i);
                                        }
                                    }
                                }
                            }
                        }
                        text_color.pop();
                        hover_color.pop();
                        active_hover_color.pop();
                    });
                    ui.window("Assets")
                    .size(
                        [window_size.0, 
                        175.0], imgui::Condition::Always
                    )
                    .position(
                        [0.0, 
                        window_size.1-175.0], 
                        imgui::Condition::Always
                    )
                    .collapsible(false)
                    .resizable(false)
                    .build(|| {
                        let text_color = ui.push_style_color(imgui::StyleColor::Text, [1.0,1.0,1.0,1.0]);
                        let hover_color = ui.push_style_color(imgui::StyleColor::HeaderHovered, [1.0,1.0,1.0,0.35]);
                        let active_hover_color = ui.push_style_color(imgui::StyleColor::HeaderActive, [1.0,1.0,1.0,0.5]);

                        if let Some(_) = ui.tab_bar("main") {
                            if let Some(_) = ui.tab_item("Project") {
                                let mut to_load = vec!();
                                let mut to_remove = vec!();

                                ui.columns(2, "layers_column", false);
                                for asset in &app.config.assets {
                                    if ui.selectable(format!("[{:#?}]{}", asset.1.type_of, asset.1.name)) {
                                        if let AssetType::Texture = asset.1.type_of {
                                            if let Some(_) = app.current_scene {
                                                ui.open_popup(asset.0);
                                            }
                                        } else if let AssetType::Script = asset.1.type_of {
                                            property_select = PropertySelect::Script;
                                        }
                                    }
                                    ui.next_column();
                                
                                    if ui.is_item_hovered() {
                                        ui.tooltip(|| {
                                            ui.text_colored([0.0, 0.0, 0.0, 1.0], format!("{:#?}", asset.1.load_type))
                                        });
                                    }

                                    let win_color = ui.push_style_color(imgui::StyleColor::PopupBg, [0.129, 0.129, 0.125, 0.9]);
                                    if let Some(_) = ui.begin_popup(asset.0) {
                                        ui.input_int2("Tile Count", &mut tile_count).build();
                                        if ui.button("Add Tilesheet") {
                                            to_load.push((asset.1.path.clone(), asset.1.absolute_path.clone()));
                                            ui.close_current_popup();
                                        }
                                    }
                                    win_color.pop();

                                    if ui.button(format!("Remove##{}", asset.0.clone())) {
                                        to_remove.push(asset.0.clone());
                                    }

                                    ui.next_column();
                                    ui.separator();
                                }

                                for load in to_load {
                                    app.add_texture(
                                        ig_renderer.gl_context(), 
                                        load.0.to_string(),
                                        load.1.to_string(), 
                                        &tile_count
                                    );
                                }

                                for rem in to_remove {
                                    app.config.assets.remove(&rem);
                                }
                            }
                            if let Some(scene) = app.current_scene.as_mut() {
                                if let Some(_) = ui.tab_item(format!("World:{}", scene.name)) {
                                    if let Some(_) = ui.tab_bar("world") {
                                        if let Some(_) = ui.tab_item("Tile Sheets") {
                                            let mut to_remove = vec!();
                                            ui.columns(2, "layers_column", false);
                                            for tex in scene.tile_sheets.iter().enumerate() {
                                                if ui.selectable(format!("{}##{}", tex.1.filename, tex.0)) {
                                                    property_select = PropertySelect::Tilesheet(tex.0);
                                                    if let Some(tilesheet) = scene.tile_sheets.get(tex.0) {
                                                        tile_count = [
                                                            (tilesheet.sheet_size.0/tilesheet.tile_size.0) as i32, 
                                                            (tilesheet.sheet_size.1/tilesheet.tile_size.1) as i32
                                                        ];
                                                    }
                                                }
                                                ui.next_column();
                                                if ui.button("Remove") {
                                                    to_remove.push(tex.0);
                                                }
                                                ui.next_column();
                                                ui.separator();
                                            }

                                            for i in to_remove {
                                                scene.tile_sheets.remove(i);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        text_color.pop();
                        hover_color.pop();
                        active_hover_color.pop();
                    });
                }

                let new_tile = if let Some(_) = app.current_scene.as_ref() {
                    if let PropertySelect::Marker(_) = property_select {
                        None
                    } else {
                        let mouse_pos = Vec2::from_slice(&ui.io().mouse_pos);

                        let model = 
                        Mat4::IDENTITY * 
                        Mat4::from_scale_rotation_translation( 
                            Vec3::new(1.0, 1.0, 1.0),
                            Quat::from_rotation_z(0.0), 
                            Vec3::new(mouse_pos.x-(window_size.0/2.0), (window_size.1/2.0)-mouse_pos.y, 0.0)
                        );

                        let view = unsafe { *crate::renderer::VIEW_MATRIX };
                        let projection = unsafe { *crate::renderer::PROJECTION_MATRIX };

                        let mvp =  model * view.inverse() * projection;
                        let position = mvp.to_scale_rotation_translation().2;
                        
                        if !ui.io().want_capture_mouse && !ui.is_key_down(imgui::Key::Space) {
                            if ui.is_mouse_down(imgui::MouseButton::Left) {
                                Some((position, true))
                            } else if ui.is_mouse_down(imgui::MouseButton::Right) {
                                Some((position, false))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                } else {
                    None
                };

                if let Some(nt) = new_tile {
                    let tile = if let Some(scene) = app.current_scene.as_ref() {
                        let sheet = scene.tile_sheets.iter().find(
                            |&a| a.path == app.get_tile_sheet()
                        );
                        
                        if let Some(sheet) = sheet {
                            if nt.1 {
                                let mut new_spr = Sprite::new(&app.current_tile_sheet);
                                new_spr.load(ig_renderer.gl_context(), &program, &app.textures);
                                new_spr.cut_sprite_sheet(0, 0,sheet.get_num_of_tiles().0, sheet.get_num_of_tiles().1);
                                new_spr.anim_sprite_sheet(
                                    ig_renderer.gl_context(), 
                                    &program, 
                                    current_tile.0 as i32, current_tile.1 as i32
                                );
                                new_spr.position = Vec2::new(
                                    sheet.tile_size.0 as f32 * f32::round(nt.0.x/sheet.tile_size.0 as f32), 
                                    sheet.tile_size.1 as f32 * f32::round(nt.0.y/sheet.tile_size.1 as f32)
                                );
                                app.sprite_buffer[app.current_layer].insert(
                                    (
                                        (sheet.tile_size.0 as f32 * f32::round(nt.0.x/sheet.tile_size.0 as f32)) as i32, 
                                        (sheet.tile_size.1 as f32 * f32::round(nt.0.y/sheet.tile_size.1 as f32)) as i32
                                    ),
                                    new_spr
                                );
                            }
                            else {
                                if let Some(layer) = scene.layers.get(app.current_layer) {
                                    if let Some(_) = layer.tiles.get(
                                        &((sheet.tile_size.0 as f32 * f32::round(nt.0.x/sheet.tile_size.0 as f32)) as i32, 
                                        (sheet.tile_size.1 as f32 * f32::round(nt.0.y/sheet.tile_size.1 as f32)) as i32)
                                    ) {
                                        app.sprite_buffer[app.current_layer].remove(
                                            &((sheet.tile_size.0 as f32 * f32::round(nt.0.x/sheet.tile_size.0 as f32)) as i32, 
                                            (sheet.tile_size.1 as f32 * f32::round(nt.0.y/sheet.tile_size.1 as f32)) as i32)
                                        );
                                    }
                                }
                            }

                            Some((Tile {
                                sheet: app.current_tile_sheet.clone(),
                                sheet_id: current_tile,
                                position: (
                                    sheet.tile_size.0 as f32 * f32::round(nt.0.x/sheet.tile_size.0 as f32), 
                                    sheet.tile_size.1 as f32 * f32::round(nt.0.y/sheet.tile_size.1 as f32)
                                )
                            }, nt.1))
                        }
                        else {
                            None
                        }
                    } else {
                        None
                    };

                    if let (Some(scene), Some(tile)) = (app.current_scene.as_mut(), tile) {
                        if tile.1 {
                            if let Some(layer) = scene.layers.get_mut(app.current_layer) {
                                layer.tiles.insert((tile.0.position.0 as i32, tile.0.position.1 as i32), tile.0);
                            }
                        }
                        else {
                            if let Some(layer) = scene.layers.get_mut(app.current_layer) {
                                layer.tiles.remove(&(tile.0.position.0 as i32, tile.0.position.1 as i32));
                            }
                        }
                    }
                }

                winit_platform.prepare_render(ui, window.window());
                let draw_data = imgui_context.render();

                ig_renderer
                    .render(draw_data)
                    .expect("error rendering imgui");
                
                window.swap_buffers().unwrap();
            }   
            glutin::event::Event::WindowEvent {
                event: glutin::event::WindowEvent::Resized(size),
                ..
            } => {
                window.resize(size);
                let logical_size: dpi::LogicalSize<f32> = size.to_logical(winit_platform.hidpi_factor());
                imgui_context.io_mut().display_size = [logical_size.width, logical_size.height];

                unsafe {
                    *crate::renderer::PROJECTION_MATRIX = 
                    Mat4::orthographic_rh_gl(
                    -window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).width/2.0, 
                    window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).width/2.0, 
                    -window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).height/2.0,
                    window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).height/2.0, 
                    1000.0, 
                    -1000.0
                    );
                }
            }
            glutin::event::Event::WindowEvent {
                event: glutin::event::WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = glutin::event_loop::ControlFlow::Exit;
            }
            event => {
                winit_platform.handle_event(imgui_context.io_mut(), window.window(), &event);
            }
        }
    });
}

fn create_window() -> (EventLoop<()>, Window) {
    let event_loop = glutin::event_loop::EventLoop::new();
    let window = glutin::window::WindowBuilder::new()
        .with_title(TITLE)
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize::new(1024, 960));
    let window = glutin::ContextBuilder::new()
        .with_vsync(true)
        .build_windowed(window, &event_loop)
        .expect("could not create window");
    let window = unsafe {
        window
            .make_current()
            .expect("could not make window context current")
    };
    (event_loop, window)
}

fn glow_context(window: &Window) -> glow::Context {
    unsafe { glow::Context::from_loader_function(|s| window.get_proc_address(s).cast()) }
}

fn imgui_init(window: &Window) -> (WinitPlatform, imgui::Context) {
    let mut imgui_context = imgui::Context::create();
    imgui_context.set_ini_filename(None);

    let mut winit_platform = WinitPlatform::init(&mut imgui_context);
    winit_platform.attach_window(
        imgui_context.io_mut(),
        window.window(),
        imgui_winit_support::HiDpiMode::Rounded,
    );

    let mut c = FontConfig::default();
    c.size_pixels = 26.0;

    imgui_context
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData { config: Some(c) }]);

    imgui_context.io_mut().font_global_scale = (1.0 / winit_platform.hidpi_factor()) as f32;

    (winit_platform, imgui_context)
}