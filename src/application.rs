extern crate pathdiff;
use glam::{Mat4, Vec3, Vec2};
use imgui::{FontId, FontConfig, ProgressBar};
use serde::{Deserialize, Serialize};
use std::{time::{Instant, Duration}, process::{Command, Stdio}, path::{Path, PathBuf}, fs::{File, self}, io::BufReader, thread, collections::HashMap, alloc::System};
use rfd::FileDialog;
use glow::{HasContext, ALWAYS, Shader};
use glutin::{event_loop::EventLoop, WindowedContext, dpi::{self, PhysicalSize}};
use imgui_winit_support::WinitPlatform;
use serde_with::serde_as;

use crate::renderer::{LilahTexture, Sprite, ShaderProgram};

const CARGO_REPLACE: &'static str = "[dependencies]\nlilah = { git = \"https://github.com/dollerama/lilah.git\" }\nrusttype = \"*\"";
const MAIN_REPLACE: &'static str = r#"
    use lilah::application::*;
    use lilah::math::Vec2;
    use lilah::world::*;

    fn setup(app : &mut App, state : &mut WorldState, scripting : &mut Scripting) {

//ASSETS
    }

    pub fn main() {  
        let mut app = App::new("Lilah", WINDOW_SIZE);
        let mut scripting = Scripting::new();

        World::new()
            .setup(Box::new(setup))
        .run(&mut app, &mut scripting);  
    }
"#;

pub enum PropertySelect {
    None,
    Layer,
    Tilesheet(usize),
    Script
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum AssetType {
    Script,
    Texture,
    Sfx,
    Music,
    Font
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum LoadType {
    External, 
    Emdedded,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub path: String,
    pub absolute_path: String,
    pub type_of: AssetType,
    pub load_type: LoadType,
    pub load_order: Option<usize>
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub assets: HashMap<String, Asset>,
    pub window_size: (f32, f32)
}

impl Config {
    pub fn new() -> Self {
        Self { assets: HashMap::new(), window_size: (800f32, 600f32) }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TileSheet {
    pub filename: String,
    pub path: String,
    pub absolute_path: String,
    pub tile_size: (u32, u32),
    pub sheet_size: (u32, u32)
}

impl TileSheet {
    pub fn get_num_of_tiles(&self) -> (u32, u32) {
        (self.sheet_size.0/self.tile_size.0, self.sheet_size.1/self.tile_size.1) 
    }
}

#[derive(Serialize, Deserialize)]
pub struct Tile {
    pub sheet: String,
    pub sheet_id: (u32, u32),
    pub position: (f32, f32)
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct Layer {
    #[serde_as(as = "Vec<(_, _)>")]
    pub tiles: HashMap<(i32, i32), Tile>,
    pub visible: bool,
    pub collision: bool,
    pub tile_sheet: String,
    pub current_tile_item: i32
}

impl Layer {
    pub fn new() -> Self {
        Self { 
            tiles: HashMap::new(),
            visible: true,
            collision: false,
            tile_sheet: String::from(""),
            current_tile_item: 0
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    pub path: String,
    pub tile_sheets: Vec<TileSheet>,
    pub layers: Vec<Layer>
}

impl Scene {
    fn new(name: &str, path: &str) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_string(),
            tile_sheets: Vec::new(),
            layers: Vec::new()
        }
    }
}

pub struct App {
    pub config: Config,
    pub current_project: String,
    pub textures: HashMap<String, LilahTexture>,
    pub current_tile_sheet: String,
    pub current_scene: Option<Scene>,
    pub current_layer: usize,
    pub sprite_buffer: Vec<HashMap<(i32, i32), Sprite>>
}

impl App {
    pub fn new() -> Self {
        Self {
            config: Config::new(),
            current_project: String::from(""),
            textures: HashMap::new(),
            current_tile_sheet: String::from(""),
            current_scene: None,
            sprite_buffer: Vec::new(),
            current_layer: 0
        }
    }

    pub fn get_tile_sheet(&self) -> String {
        self.current_tile_sheet.clone()
    }

    pub fn load_texture(&mut self, gl: &glow::Context, file : &str) {
        let mut new_texture = unsafe { 
            LilahTexture::new(gl) 
        };

        unsafe {
            new_texture.set_wrapping(gl, glow::REPEAT as i32);
            new_texture.set_filtering(gl, glow::LINEAR as i32);
        }

        unsafe {
            if let Err(e) = new_texture.load(gl, &Path::new(file)) {
                eprintln!("{}", e);
            }
        }

        //let file = String::from(Path::new(file).file_name().unwrap().to_str().unwrap());

        self.textures.insert(file.to_string(), new_texture);
    }

    pub fn write_config(&self) {
        let _ = fs::write(
            format!("{}/config.json", 
            self.current_project).as_str(), 
            serde_json::to_string(&self.config).unwrap()
        );
    }

    pub fn write_current_scene(&self) {
        if let Some(scene) = self.current_scene.as_ref() {
            let _ = fs::write(
                format!("{}/{}", self.current_project, scene.path),
                serde_json::to_string(&scene).unwrap()
            );
        }
    }
 
    pub fn open_scene(&mut self, gl: &glow::Context, program: &ShaderProgram) {
        if let Some(file) = FileDialog::new()
        .set_directory(format!("{}", self.current_project))
        .pick_file() {
            let path_base = Path::new(&self.current_project);
            let file_path_str = file.as_path().to_str().unwrap();
            let path = Path::new(file_path_str);
            let relative_path_to = pathdiff::diff_paths(path, path_base).unwrap();
            let file_path = relative_path_to.as_path();

            match fs::read(file) {
                Ok(v) => {
                    self.current_scene = Some(serde_json::from_slice(&v).unwrap());
                }
                Err(_) => {
                }
            } 

            if let Some(scene) = self.current_scene.as_ref() {
                let mut to_load = vec!();
                for i in &scene.tile_sheets {
                    to_load.push(i.path.clone());
                }
                for i in to_load {
                    self.load_texture(gl, &i);
                }
            }

            if let Some(scene) = self.current_scene.as_ref() {
                self.current_tile_sheet = scene.layers[0].tile_sheet.clone();

                let mut tiles = vec!();
                self.sprite_buffer.clear();
                for j in scene.layers.iter().enumerate() {
                    for i in &j.1.tiles {
                        tiles.push((j.0, i.clone()));
                    }
                    self.sprite_buffer.push(HashMap::new());
                }
                
                for i in tiles {
                    let mut new_spr = Sprite::new(&i.1.1.sheet);
                    new_spr.load(gl, &program, &self.textures);
                    new_spr.cut_sprite_sheet(0, 0, 3, 3);
                    new_spr.anim_sprite_sheet(
                        gl, 
                        &program,  
                        i.1.1.sheet_id.0 as i32, i.1.1.sheet_id.1 as i32
                    );
                    new_spr.position = Vec2::new(
                        i.1.0.0 as f32, 
                        i.1.0.1 as f32
                    );

                    self.sprite_buffer[i.0].insert(* i.1.0, new_spr);
                }
            }
        }
    }

    pub fn new_scene(&mut self) {
        if let Some(file) = FileDialog::new()
        .set_directory(format!("{}", self.current_project))
        .save_file() {
            let file_path_str = file.as_path().to_str().unwrap();

            let path_base = Path::new(&self.current_project);
            let path = Path::new(file_path_str);
            let relative_path_to = pathdiff::diff_paths(path, path_base).unwrap();

            let file_name = relative_path_to.as_path().file_name().unwrap().to_str().unwrap();
            let file_path = relative_path_to.as_path().to_str().unwrap();

            let mut new_scene = Scene::new(file_name, &format!("{}.json", file_path));
            new_scene.layers.push(Layer::new());
            self.sprite_buffer.push(HashMap::new());

            let _ = fs::write(
                format!("{}.json", file.as_path().to_str().unwrap()),
                serde_json::to_string(&new_scene).unwrap()
            );

            self.current_scene = Some(new_scene);

            self.write_config();
        }
    }

    pub fn new_project(&mut self) -> &str {
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

            self.write_config();
        }

        &self.current_project
    }

    pub fn open_project(&mut self) -> &str {
        if let Some(file) = FileDialog::new()
        .set_directory("/")
        .pick_folder() {
            self.current_project = file.as_path().to_str().unwrap().to_string();

            match fs::read(format!("{}/config.json", self.current_project).as_str()) {
                Ok(v) => {
                    self.config = serde_json::from_slice(&v).unwrap();
                }
                Err(_) => {
                    self.write_config();
                }
            }
        }

        &self.current_project
    }

    pub fn wrangle_main(&self) {
        let mut assets_str = String::from("");
        let mut sorted_scripts = vec!();
        for asset in &self.config.assets {
            match asset.1.type_of {
                AssetType::Script => {
                    match asset.1.load_type {
                        LoadType::Emdedded => {
                            // assets_str.push_str(
                            //     format!("\t\tembed_script!(\"{}\", scripting);\n", 
                            //     asset.1.path).as_str()
                            // )
                            sorted_scripts.push((asset.1.load_order.unwrap(), format!("\t\tembed_script!(\"{}\", scripting);\n", asset.1.path)));
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

        sorted_scripts.sort_by(|a, b| a.0.cmp(&b.0));

        for s in sorted_scripts {
            assets_str.push_str(&s.1);
        }
        
        let _ = fs::write(format!("{}/src/main.rs", self.current_project), MAIN_REPLACE);
        let main_file = fs::read_to_string(format!("{}/src/main.rs", self.current_project)).unwrap();
        let _ = fs::write(
            format!("{}/src/main.rs", self.current_project), 
            main_file.replace("//ASSETS", &assets_str).replace("WINDOW_SIZE", format!("Vec2.new({}, {})", self.config.window_size.0, self.config.window_size.1).as_str())
        );

        while fs::read_to_string(format!("{}/src/main.rs", self.current_project)).is_err() {
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn run_project(&mut self) {
        self.wrangle_main();

        let status = Command::new( "cargo" )
        .args(["run", "--manifest-path", format!("{}/Cargo.toml", self.current_project).as_str()])
        .spawn().expect("failed");
    }

    pub fn add_texture(&mut self, gl: &glow::Context, abs_path: String, path: String, tile_count: &[i32; 2]) {
        self.load_texture(gl, &path);

        let size = self.textures.get(&path).unwrap().size;

        let filename_split = path.split("/").collect::<Vec<&str>>();
        let filename = filename_split[filename_split.len()-1];

        if let Some(scene) = self.current_scene.as_mut() {
            scene.tile_sheets.push(
                TileSheet { 
                    filename: filename.to_string(),
                    absolute_path: abs_path, 
                    path: path,
                    tile_size: ((size.x/tile_count[0] as f32) as u32, (size.y/tile_count[1] as f32) as u32), 
                    sheet_size: (size.x as u32, size.y as u32) 
                }
            );
        }

        self.write_current_scene();
        self.write_config();
    }

    pub fn add_external_asset(&mut self) {
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
                    name: file.file_name().unwrap().to_str().unwrap().to_string(),
                    path: relative_path_to.expect("Path").as_path().to_str().unwrap().to_string(),
                    absolute_path: file.as_os_str().to_str().unwrap().to_string(),
                    type_of: type_of,
                    load_type: LoadType::External,
                    load_order: None
                };


                self.config.assets.insert(format!("{}_{:?}", a.path.clone(), LoadType::External), a);

                self.write_config();
            }
        }
    }

    pub fn add_embedded_asset(&mut self) {
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
                //let path_base = Path::new(&self.current_project);
                let path_base_from_src = Path::new(&script_base);
                let file_path_str = file.as_path().to_str().unwrap();
                let path = Path::new(file_path_str);
                let relative_path_to = pathdiff::diff_paths(path, path_base_from_src);

                let mut order = -1;
                if let AssetType::Script = type_of {
                    order = 0;
                    for i in &self.config.assets {
                        if let AssetType::Script = i.1.type_of {
                            order += 1;
                        } 
                    }
                }
                let order_to_use = if order == -1 {
                    None
                } else {
                    Some(order as usize)
                };

                let a =
                Asset {
                    name: file.file_name().unwrap().to_str().unwrap().to_string(),
                    path: relative_path_to.expect("Path").as_path().to_str().unwrap().to_string(),
                    absolute_path: file.as_os_str().to_str().unwrap().to_string(),
                    type_of: type_of,
                    load_type: LoadType::Emdedded,
                    load_order: order_to_use
                };

                self.config.assets.insert(format!("{}_{:?}", a.path.clone(), LoadType::Emdedded), a);

                self.write_config();
            }
        }
    }
}