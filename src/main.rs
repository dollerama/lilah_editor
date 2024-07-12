extern crate pathdiff;
use application::{App, AssetType, Tile, Layer, TileSheet, PropertySelect};
use glam::{Mat4, Vec3, Vec2, Quat};
use imgui::{FontConfig, Selectable, TextureId};
use renderer::{ShaderProgram, Sprite};
use std::{time::Instant, collections::HashMap};
use glow::HasContext;
use glutin::{event_loop::EventLoop, WindowedContext, dpi, event::{ElementState, KeyboardInput, VirtualKeyCode}};
use imgui_winit_support::WinitPlatform;

mod renderer;
mod application;

const TITLE: &str = "Lilah Editor";

type Window = WindowedContext<glutin::PossiblyCurrent>;

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
            0.0, 
            window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).width, 
            0.0,
            window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).height, 
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

    let mut camera = Vec2::new(0.0, 0.0);
    let mut tile_count = [0, 0];
    let mut current_tile = (0u32, 0u32);
    let mut property_select = PropertySelect::None;

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
                let mut ui_hovered: bool = false;
                let ui = imgui_context.frame();
                let window_size = (
                    window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).width,
                    window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).height
                );
                
                if ui.is_key_down(imgui::Key::Space) {
                    let drag = ui.mouse_drag_delta_with_button(imgui::MouseButton::Left);
                    camera += Vec2::new(-drag[0], drag[1])*0.05;
                    unsafe {
                        *crate::renderer::VIEW_MATRIX = 
                        Mat4::from_translation(Vec3::new(-camera.x, -camera.y, 0.0));
                    }
                }

                unsafe { ig_renderer.gl_context().clear(glow::COLOR_BUFFER_BIT) };

                for sprs in &app.sprite_buffer {
                    for spr in sprs {
                        spr.1.draw(ig_renderer.gl_context(), &program, &app.textures);
                    }
                }
                
                if let Some(main_menu) = ui.begin_main_menu_bar() {
                    if let Some(_) = ui.begin_menu("File") {
                        if ui.menu_item("New") {
                            window.window().set_title(app.new_project());
                        }
                        if ui.menu_item("Open") {
                            window.window().set_title(app.open_project());
                        }
                    }
                    if app.current_project != "" {
                        if let Some(_) = ui.begin_menu("Project") {
                            if ui.menu_item("Run") {
                                app.run_project();
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
                            if let Some(scene) = app.current_scene.as_ref() {
                                if ui.menu_item("Save") {
                                    app.write_current_scene();
                                }
                            }
                        }
                    }
                    main_menu.end();
                }
                
                if app.current_project != "" {
                    if let Some(_) = app.current_scene.as_ref() {
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
                            match property_select {
                                PropertySelect::None => {}
                                PropertySelect::Tilesheet(sheet) => {
                                    if let Some(_) = ui.tab_bar("prop_main") {
                                        if let Some(_) = ui.tab_item("Tile Sheet") {
                                            if let Some(scene) = app.current_scene.as_mut() {
                                                if let Some(tilesheet) = scene.tile_sheets.get_mut(sheet) {
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
                                                            ui.open_popup("TileSheetPopup")
                                                        }
                                                    }
                                                    ui.next_column();
                                                    ui.text("Tile Count");
                                                    ui.next_column();
                                                    ui.text(format!("{}", layer.tiles.keys().len()));
                                                    ui.next_column();

                                                    if let Some(_) = ui.begin_popup("TileSheetPopup") { 
                                                        let list = scene.tile_sheets.iter().map(|TileSheet { ref filename, .. }| filename.as_str()).collect::<Vec<&str>>();
                                                        let list2 = scene.tile_sheets.iter().map(|TileSheet { ref path, .. }| path.as_str()).collect::<Vec<&str>>();
                                                        if ui.list_box("Tile Sheet", &mut layer.current_tile_item, list.as_slice(), list.len() as i32) {
                                                            layer.tile_sheet = list2[layer.current_tile_item as usize].to_string();
                                                            app.current_tile_sheet = list2[layer.current_tile_item as usize].to_string();
                                                            println!("{}", app.current_tile_sheet);

                                                            for j in &mut layer.tiles {
                                                                if let Some(buffer) = app.sprite_buffer.get_mut(app.current_layer) {
                                                                    if let Some(spr) = buffer.get_mut(
                                                                        j.0
                                                                    ) {
                                                                        spr.texture_id = list2[layer.current_tile_item as usize].to_string();
                                                                    }
                                                                }
                                                            } 
                                                            ui.close_current_popup();
                                                        }
                                                    }
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
                            }
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
                            if let Some(_) = ui.tab_bar("main") {
                                if let Some(_) = ui.tab_item("Layers") {
                                    if let Some(scene) = app.current_scene.as_mut() {
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
                                                } 
                                            }
                                            ui.next_column();
                                            let button_label = if i.1.visible {
                                                format!("Hide {}", i.0)
                                            }
                                            else {
                                                format!("Show {}", i.0)
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
                                    }
                                }
                            }
                        });
                    }
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
                        ui_hovered = ui.is_any_item_hovered() || ui.is_window_hovered();

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
                                        }
                                    }
                                    ui.next_column();
                                
                                    if ui.is_item_hovered() {
                                        ui.tooltip(|| {
                                            ui.text_colored([1.0, 1.0, 1.0, 1.0], format!("{:#?}", asset.1.load_type))
                                        });
                                    }

                                    if let Some(_) = ui.begin_popup(asset.0) {
                                        ui.input_int2("Tile Count", &mut tile_count).build();
                                        if ui.button("Add Tilesheet") {
                                            to_load.push((asset.1.path.clone(), asset.1.absolute_path.clone()));
                                            ui.close_current_popup();
                                        }
                                    }

                                    if ui.button("Remove") {
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
                            if let Some(scene) = app.current_scene.as_ref() {
                                if let Some(_) = ui.tab_item(format!("World > {}", scene.name)) {
                                    if let Some(_) = ui.tab_bar("world") {
                                        if let Some(_) = ui.tab_item("Tile Sheets") {
                                            let mut to_remove = vec!();
                                            ui.columns(2, "layers_column", false);
                                            for tex in app.textures.keys().enumerate() {
                                                if ui.selectable(tex.1) {
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
                                                    to_remove.push(tex.1.clone());
                                                }
                                                ui.next_column();
                                                ui.separator();
                                            }

                                            for i in to_remove {
                                                app.textures.remove(&i);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });
                }

                let new_tile = if let Some(_) = app.current_scene.as_ref() {
                    let mouse_pos = Vec2::from_slice(&ui.io().mouse_pos);

                    let model = 
                    Mat4::IDENTITY * 
                    Mat4::from_scale_rotation_translation( 
                        Vec3::new(1.0, 1.0, 1.0),
                        Quat::from_rotation_z(0.0), 
                        Vec3::new(mouse_pos.x, window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).height-mouse_pos.y, 0.0)
                    );

                    let view = unsafe { *crate::renderer::VIEW_MATRIX };
                    let projection = unsafe { *crate::renderer::PROJECTION_MATRIX };

                    let mvp =  model * view.inverse() * projection;
                    let position = mvp.to_scale_rotation_translation().2;
                    
                    if !ui_hovered && !ui.is_key_down(imgui::Key::Space) {
                        if ui.is_mouse_down(imgui::MouseButton::Left) {
                            Some((position, true))
                        }
                        else if ui.is_mouse_down(imgui::MouseButton::Right) {
                            Some((position, false))
                        }
                        else {
                            None
                        }
                    }
                    else {
                        None
                    }
                }
                else {
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
                                new_spr.cut_sprite_sheet(0, 0, 3, 3);
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
                        0.0, 
                        window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).width, 
                        0.0,
                        window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).height, 
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