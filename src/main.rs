extern crate pathdiff;
use application::{App, AssetType};
use glam::{Mat4, Vec3, Vec2};
use imgui::{FontConfig, Selectable};
use renderer::{ShaderProgram, Sprite};
use std::time::Instant;
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

    let mut ig_renderer = imgui_glow_renderer::AutoRenderer::initialize(gl, &mut imgui_context)
        .expect("failed to create renderer"); 

    unsafe {
        *crate::renderer::PROJECTION_MATRIX = 
            Mat4::orthographic_rh_gl(
                0.0, 
                window.window().inner_size().to_logical(winit_platform.hidpi_factor()).width, 
                0.0,
                window.window().inner_size().to_logical(winit_platform.hidpi_factor()).height, 
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

    let vert = unsafe { renderer::Shader::new(ig_renderer.gl_context(), crate::renderer::DEFAULT_VERT, glow::VERTEX_SHADER).unwrap() };
    let frag = unsafe { renderer::Shader::new(ig_renderer.gl_context(), crate::renderer::DEFAULT_FRAG, glow::FRAGMENT_SHADER).unwrap() };
    let program = unsafe { ShaderProgram::new(ig_renderer.gl_context(), &[vert, frag]).unwrap() };

    let mut spr: Option<Sprite> = None;

    let mut camera = Vec2::new(0.0, 0.0);

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
                let drag = ui.mouse_drag_delta_with_button(imgui::MouseButton::Left);
                camera += Vec2::new(-drag[0], drag[1])*0.05;
                unsafe {
                    *crate::renderer::VIEW_MATRIX = 
                    Mat4::from_translation(Vec3::new(-camera.x, -camera.y, 0.0));
                }

                unsafe { ig_renderer.gl_context().clear(glow::COLOR_BUFFER_BIT) };

                if let Some(s) = &spr {
                    s.draw(ig_renderer.gl_context(), &program, &app.textures);
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
                            if let Some(_) = ui.begin_menu("Entities") {
                                if ui.menu_item("Sprite") {
                                    spr = Some(Sprite::new(&app.get_tile_sheet()));
                                    spr.as_mut().expect("Failed").load(ig_renderer.gl_context(), &program, &app.textures);
                                    spr.as_mut().expect("Failed").position = Vec2::new(200.0,300.0);
                                }
                            }
                        }
                        if let Some(_) = ui.begin_menu("World") {
                            if ui.menu_item("New") {
                                app.new_scene();
                            }
                            if ui.menu_item("Open") {
                                app.open_scene(ig_renderer.gl_context());
                            }
                        }
                    }
                    main_menu.end();
                }
                
                if app.current_project != "" {
                    ui.window("Assets")
                    .size([window.window().inner_size().to_logical(winit_platform.hidpi_factor()).width, 150.0], imgui::Condition::FirstUseEver)
                    .position([0.0, window.window().inner_size().to_logical::<f32>(winit_platform.hidpi_factor()).height as f32-150.0], imgui::Condition::FirstUseEver)
                    .always_vertical_scrollbar(true)
                    .build(|| {
                        if let Some(_) = ui.tab_bar("main") {
                            if let Some(_) = ui.tab_item("Project") {
                                let mut to_load = vec!();

                                for asset in &app.config.assets {
                                    if ui.selectable(format!("[{:#?}]{}", asset.1.type_of, asset.1.name)) {
                                        if let AssetType::Texture = asset.1.type_of {
                                            if app.current_scene != "" {
                                                ui.open_popup(asset.0);
                                            }
                                        }
                                    }

                                    if let Some(_) = ui.begin_popup(asset.0) {
                                        if ui.button("Add Tilesheet") {
                                            println!("{}", asset.1.absolute_path);
                                            to_load.push(asset.1.absolute_path.clone());
                                            ui.close_current_popup();
                                        }
                                    }

                                    if ui.is_item_hovered() {
                                        ui.tooltip(|| {
                                            ui.text_colored([1.0, 1.0, 1.0, 1.0], format!("{:#?}", asset.1.load_type))
                                        });
                                    }
                                    ui.separator();
                                }

                                for load in &to_load {
                                    app.add_texture(ig_renderer.gl_context(), load.to_string());
                                }
                            }
                            if app.current_scene != "" {
                                if let Some(_) = ui.tab_item(format!("World > {}", app.current_scene)) {
                                    for tex in &app.textures {
                                        if ui.selectable(tex.0) {
                                            app.current_tile_sheet = tex.0.to_string();
                                        }
                                    }
                                }
                            }
                        }
                    });
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