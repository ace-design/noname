use super::host_functions::FUNCTIONS;
use extism::Plugin;
use std::{env, fs};

pub struct PluginManager {
    plugins: Vec<Plugin<'static>>,
}

impl PluginManager {
    pub fn new() -> PluginManager {
        PluginManager {
            plugins: Vec::new(),
        }
    }

    pub fn load_plugins(&mut self) {
        info!("Loading plugins");
        self.plugins = Vec::new();

        if let Some(mut home_path) = env::var_os("HOME") {
            home_path.push("/.config/p4_lsp/plugins/");

            let paths = match fs::read_dir(&home_path) {
                Ok(paths) => paths,
                Err(_) => {
                    error!("Couldn't read from plugins path ({:?}).", home_path);
                    return;
                }
            };

            for path in paths {
                if let Ok(dir_entry) = path {
                    info!("Loading plugin: {}", dir_entry.path().display());
                    let file_content = fs::read(dir_entry.path()).unwrap();
                    let functions = (*FUNCTIONS).clone();

                    match Plugin::create(file_content, functions, true) {
                        Ok(plugin) => {
                            self.plugins.push(plugin);
                        }
                        Err(err) => {
                            error!(
                                "Failed loading plugin: {} Error: {}",
                                dir_entry.path().display(),
                                err
                            );
                        }
                    }
                }
            }
        }

        info!("Loaded {} plugin(s)", self.plugins.len());
    }

    pub fn run_plugins(&mut self) {
        for plugin in &mut self.plugins {
            let result = plugin.call("count_vowels", "testing");
            if let Ok(output) = result {
                info!(
                    "Plugin called: {}",
                    String::from_utf8(output.to_vec()).expect("Invalid string")
                );
            }
        }
    }
}
