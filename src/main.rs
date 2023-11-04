extern crate pathdiff;
use imgui::{FontId, FontConfig, ProgressBar};
use serde::{Deserialize, Serialize};
use std::{time::{Instant, Duration}, process::{Command, Stdio}, path::{Path, PathBuf}, fs::{File, self}, io::BufReader, thread, collections::HashMap, alloc::System};
use rfd::FileDialog;
use glow::{HasContext, ALWAYS};
use glutin::{event_loop::EventLoop, WindowedContext};
use imgui_winit_support::WinitPlatform;

const TITLE: &str = "Lilah Editor";

type Window = WindowedContext<glutin::PossiblyCurrent>;

const CARGO_REPLACE: &'static str = "[dependencies]\nlilah = { git = \"https://github.com/dollerama/lilah.git\" }\nrusttype = \"*\"";
const MAIN_REPLACE: &'static str = r#"
    use lilah::application::*;
    use lilah::math::Vec2;
    use lilah::world::*;

    fn setup(app : &mut App, state : &mut WorldState, scripting : &mut Scripting) {

//ASSETS
    }

    pub fn main() {  
        let mut app = App::new("Lilah", Vec2::new(800.0, 600.0));
        let mut scripting = Scripting::new();

        World::new()
            .setup(Box::new(setup))
        .run(&mut app, &mut scripting);  
    }
"#;

#[derive(Serialize, Deserialize, Debug)]
enum AssetType {
    Script,
    Texture,
    Sfx,
    Music,
    Font
}

#[derive(Serialize, Deserialize, Debug)]
enum LoadType {
    External, 
    Emdedded,
}

#[derive(Serialize, Deserialize)]
struct Asset {
    path: String,
    type_of: AssetType,
    load_type: LoadType
}

#[derive(Serialize, Deserialize)]
struct Config {
    pub assets: HashMap<String, Asset>
}

impl Config {
    fn new() -> Self {
        Self { assets: HashMap::new() }
    }
}

struct App {
    config: Config,
    current_project: String
}

impl App {
    fn new() -> Self {
        Self {
            config: Config::new(),
            current_project: String::from("")
        }
    }

    fn new_project(&mut self) -> &str {
        if let Some(file) = FileDialog::new()
        .set_directory("/")
        .save_file() {
            Command::new( "cargo" )
            .args(["new", file.as_path().to_str().unwrap().clone()])
            .spawn( )
            .unwrap( ); 

            self.current_project = file.as_path().to_str().unwrap().to_string();

            while fs::read_to_string(format!("{}/Cargo.toml", self.current_project)).is_err() {
                thread::sleep(Duration::from_millis(10));
            }

            let cargo_file = fs::read_to_string(format!("{}/Cargo.toml", self.current_project)).unwrap();
            let _ = fs::write(
                format!("{}/Cargo.toml", self.current_project),
                cargo_file.replace("[dependencies]", CARGO_REPLACE)
            );

            let _ = fs::write(format!("{}/src/main.rs", self.current_project), MAIN_REPLACE);
            let _ = fs::create_dir(format!("{}/assets", self.current_project));
            let _ = fs::create_dir(format!("{}/src/scripts", self.current_project));
            let _ = fs::create_dir(format!("{}/src/assets", self.current_project));

            let _ = fs::write(
                format!("{}/config.json", 
                self.current_project).as_str(), 
                serde_json::to_string(&self.config).unwrap()
            );
        }

        &self.current_project
    }

    fn open_project(&mut self) -> &str {
        if let Some(file) = FileDialog::new()
        .set_directory("/")
        .pick_folder() {
            self.current_project = file.as_path().to_str().unwrap().to_string();

            match fs::read(format!("{}/config.json", self.current_project).as_str()) {
                Ok(v) => {
                    self.config = serde_json::from_slice(&v).unwrap();
                }
                Err(_) => {
                    let _ = fs::write(
                        format!("{}/config.json", 
                        self.current_project).as_str(), 
                        serde_json::to_string(&self.config).unwrap()
                    );
                }
            }
        }

        &self.current_project
    }

    fn wrangle_main(&self) {
        let mut assets_str = String::from("");
        for asset in &self.config.assets {
            match asset.1.type_of {
                AssetType::Script => {
                    match asset.1.load_type {
                        LoadType::Emdedded => {
                            assets_str.push_str(
                                format!("\t\tembed_script!(\"{}\", scripting);\n", 
                                asset.1.path).as_str()
                            )
                        }
                        LoadType::External => {
                            panic!("Script cannot be external");
                        }
                    }
                }
                AssetType::Texture => {
                    match asset.1.load_type {
                        LoadType::Emdedded => {
                            assets_str.push_str(
                                format!("\t\tembed_texture!(\"{}\", state, app);\n", 
                                asset.1.path).as_str()
                            )
                        }
                        LoadType::External => {
                            assets_str.push_str(
                                format!("\t\tload_texture!(\"{}\", state, app);\n", 
                                asset.1.path).as_str()
                            )
                        }
                    }
                }
                AssetType::Sfx => {
                    match asset.1.load_type {
                        LoadType::Emdedded => {
                            assets_str.push_str(
                                format!("\t\tembed_sfx!(\"{}\", state);\n", 
                                asset.1.path).as_str()
                            )
                        }
                        LoadType::External => {
                            assets_str.push_str(
                                format!("\t\tload_sfx!(\"{}\", state);\n", 
                                asset.1.path).as_str()
                            )
                        }
                    }
                }
                AssetType::Music => {
                    match asset.1.load_type {
                        LoadType::Emdedded => {
                            assets_str.push_str(
                                format!("\t\tembed_music!(\"{}\", state);\n", 
                                asset.1.path).as_str()
                            )
                        }
                        LoadType::External => {
                            assets_str.push_str(
                                format!("\t\tload_music!(\"{}\", state);\n", 
                                asset.1.path).as_str()
                            )
                        }
                    }
                }
                AssetType::Font => {
                    match asset.1.load_type {
                        LoadType::Emdedded => {
                            assets_str.push_str(
                                format!("\t\tembed_font!(\"{}\", state);\n", 
                                asset.1.path).as_str()
                            )
                        }
                        LoadType::External => {
                            panic!("Script cannot be external");
                        }
                    }
                }
            }
        }
        
        let _ = fs::write(format!("{}/src/main.rs", self.current_project), MAIN_REPLACE);
        let main_file = fs::read_to_string(format!("{}/src/main.rs", self.current_project)).unwrap();
        let _ = fs::write(
            format!("{}/src/main.rs", self.current_project), 
            main_file.replace("//ASSETS", &assets_str)
        );

        while fs::read_to_string(format!("{}/src/main.rs", self.current_project)).is_err() {
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn run_project(&mut self) {
        self.wrangle_main();

        let status = Command::new( "cargo" )
        .args(["run", "--manifest-path", format!("{}/Cargo.toml", self.current_project).as_str()])
        .spawn().expect("failed");
    }

    fn add_external_asset(&mut self) {
        if let Some(files) = FileDialog::new()
        .set_directory(format!("{}", self.current_project))
        .add_filter("Type", &["png", "wav", "mp3"])
        .pick_files() {
            for file in &files {
                let type_of = match file.extension().unwrap().to_str() {
                    Some("wren") => {
                        return;
                    }
                    Some("png") => {
                        AssetType::Texture
                    }
                    Some("wav") => {
                        AssetType::Sfx
                    }
                    Some("mp3") => {
                        AssetType::Music
                    }
                    Some("ttf") => {
                        return;
                    }
                    Some(&_) => todo!(),
                    None => todo!(),
                };

                let path_base = Path::new(&self.current_project);
                let file_path_str = file.as_path().to_str().unwrap();
                let path = Path::new(file_path_str);
                let relative_path_to = pathdiff::diff_paths(path, path_base);

                let a =
                Asset {
                    path: relative_path_to.expect("Path").as_path().to_str().unwrap().to_string(),
                    type_of: type_of,
                    load_type: LoadType::Emdedded
                };


                self.config.assets.insert(file.file_name().unwrap().to_str().unwrap().to_string(), a);

                let _ = fs::write(
                    format!("{}/config.json", 
                    self.current_project).as_str(), 
                    serde_json::to_string(&self.config).unwrap()
                );
            }
        }
    }

    fn add_embedded_asset(&mut self) {
        if let Some(files) = FileDialog::new()
        .set_directory(format!("{}", self.current_project))
        .add_filter("Type", &["png", "wav", "mp3", "wren", "ttf"])
        .pick_files() {
            for file in &files {
                let type_of = match file.extension().unwrap().to_str() {
                    Some("wren") => {
                        AssetType::Script
                    }
                    Some("png") => {
                        AssetType::Texture
                    }
                    Some("wav") => {
                        AssetType::Sfx
                    }
                    Some("mp3") => {
                        AssetType::Music
                    }
                    Some("ttf") => {
                        AssetType::Font
                    }
                    Some(&_) => todo!(),
                    None => todo!(),
                };

                let script_base = format!("{}/src", self.current_project);
                let path_base = Path::new(&self.current_project);
                let path_base_from_src = Path::new(&script_base);
                let file_path_str = file.as_path().to_str().unwrap();
                let path = Path::new(file_path_str);
                let relative_path_to = pathdiff::diff_paths(path, path_base_from_src);

                let a =
                Asset {
                    path: relative_path_to.expect("Path").as_path().to_str().unwrap().to_string(),
                    type_of: type_of,
                    load_type: LoadType::Emdedded
                };


                self.config.assets.insert(file.file_name().unwrap().to_str().unwrap().to_string(), a);

                let _ = fs::write(
                    format!("{}/config.json", 
                    self.current_project).as_str(), 
                    serde_json::to_string(&self.config).unwrap()
                );
            }
        }
    }
}

fn main() {
    let mut app = App::new();

    let (event_loop, window) = create_window();
    let (mut winit_platform, mut imgui_context) = imgui_init(&window);

    let gl = glow_context(&window);

    let mut ig_renderer = imgui_glow_renderer::AutoRenderer::initialize(gl, &mut imgui_context)
        .expect("failed to create renderer");

    let mut last_frame = Instant::now();

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
                unsafe { ig_renderer.gl_context().clear(glow::COLOR_BUFFER_BIT) };

                let ui = imgui_context.frame();
                
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
                    }
                    main_menu.end();
                }
                
                if app.current_project != "" {
                    ui.window("Assets")
                    .size([800.0, 100.0], imgui::Condition::Always)
                    .position([0.0, 500.0], imgui::Condition::Always)
                    .always_vertical_scrollbar(true)
                    .build(|| {
                        for asset in &app.config.assets {
                            ui.text(format!("[{:#?}]{}", asset.1.type_of, asset.0));
                            if ui.is_item_hovered() {
                                ui.tooltip(|| {
                                    ui.text_colored([1.0, 1.0, 1.0, 1.0], format!("{:#?}", asset.1.load_type));
                                });
                            }
                            ui.separator();
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
        .with_inner_size(glutin::dpi::LogicalSize::new(800, 600));
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