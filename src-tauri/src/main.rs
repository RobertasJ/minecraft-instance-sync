// #![cfg_attr(
//     all(not(debug_assertions), target_os = "windows"),
//     windows_subsystem = "windows"
// )]
#![allow(unused)]

use std::{env, fs};

use crossterm::style::Stylize;
use prelude::*;

use crate::color::*;

mod config;
mod dirs;
mod prelude;
mod git;
mod execute;
mod color;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn folder_list() -> Vec<String> {
    let config = config::open_or_else_create();
    config.folders.keys().cloned().collect()
}

#[tauri::command]
fn add_folder(name: String) {
    let mut config = config::open_or_else_create();
    config.folders.insert(name, config::Folder::default());

    config::save(&config);
}

#[tauri::command]
fn get_folder(name: String) -> Option<config::Folder> {
    let config = config::open_or_else_create();
    config.folders.get(&name).cloned()
}

#[tauri::command]
fn delete_folder(name: String) {
    let mut config = config::open_or_else_create();
    config.folders.remove(&name);
    config::save(&config);
}

#[tauri::command]
fn update_folder(current_folder: String, folder: config::Folder) {
    let mut config = config::open_or_else_create();
    config.folders.insert(current_folder, folder);
    config::save(&config);
}

#[tauri::command]
async fn sync_folder(name: String) {
    let folder = get_folder(name).unwrap();
    if folder.sync {
        let repo = folder.repo.unwrap();
        let branch = folder.branch.unwrap();
        let exec_dir = &folder.path.clone().unwrap();
        println!("{}", exec_dir.display());
        env::set_current_dir(exec_dir).unwrap();
        if git::is_repo(&folder.path.unwrap()) {

            println!(" ");
            println!(" ");
            println!("{}", "Checking for updates.".green());

            if git::current_repo(exec_dir).expect("not a git repo")!= repo {
                println!("{}", &"Current repository doesnt match the repository in the config file.".tty_red().tty_bold());
                #[cfg(target_os = "windows")] {
                    execute::no_output("powershell remove-item -recurse -force .git").unwrap();
                }
                #[cfg(not(target_os = "windows"))] {
                    execute::no_output("rm -rf .git")?;
                }
                env::set_current_dir(exec_dir).unwrap();

                let msg = format!("{}{}{}", 
                "Cloning git repo into ".tty_green(),
                exec_dir.to_str().unwrap().tty_bold(),
                r#". Remember to not have any characters like "(" or ")" 
in your path to the instance otherwise powershell will eat shit and die. 
You can remove the tmp folder after the script is complete."#).tty_green();
                println!(" ");
                println!(" ");
                println!("{}", &msg.tty_green());

                execute::no_output("git init").expect("failed to run git");
                execute::no_output(&format!("git remote add origin {}", repo)).expect("failed to run git");
                execute::no_output("git add *").expect("failed to run git");
                execute::no_output("git commit -a -m \"tmp commit\"").expect("failed to run git");
                execute::color(&format!("git fetch origin {}", branch)).expect("failed to run git");
                execute::color(&format!(" git merge -s recursive -X theirs origin/{}", branch)).expect("failed to run git");
                execute::no_output(&format!("git switch {}", branch)).expect("failed to run git");
            }
            execute::no_output("git add *").expect("failed to run git");
            execute::no_output("git commit -a -m \"tmp commit\"").expect("failed to run git");
            execute::color(&format!("git fetch origin {}", branch)).expect("failed to run git");
            execute::color(&format!(" git merge -s recursive -X theirs origin/{}", branch)).expect("failed to run git");
            execute::no_output(&format!("git switch {}", branch)).expect("failed to run git");
        } else {
            let msg = format!("{}{}{}", 
            "Cloning git repo into ".tty_green(),
            exec_dir.to_str().unwrap().tty_green().tty_bold(),
            ". Remember to not have any characters like \"(\" or \")\" \nin your path to the instance otherwise powershell will eat shit and die. \nYou can remove the tmp folder after the script is complete.").green();
            println!(" ");
            println!(" ");
            println!("{}", &msg.tty_green());

            execute::no_output("git init").expect("failed to run git");
            execute::no_output(&format!("git remote add origin {}", repo)).expect("failed to run git");
            execute::no_output("git add *").expect("failed to run git");
            execute::no_output("git commit -a -m \"tmp commit\"").expect("failed to run git");
            execute::color(&format!("git fetch origin {}", branch)).expect("failed to run git");
            execute::color(&format!(" git merge -s recursive -X theirs origin/{}", branch)).expect("failed to run git");
            execute::no_output(&format!("git switch {}", branch)).expect("failed to run git");
        }
        if folder.run_instancesync {
            let server = folder.server;

            let msg = "Launching instancesync. It will always find removed mods if there any any mods in the localMods or/and offlineMods folders. \nThey automatically get copied back over in the next step which is the intended way for having them up to date with the repo.";
            println!(" ");
            println!(" ");
            println!("{}", msg.tty_green());
            execute::color("java -jar instancesync.jar").expect("Failed to launch instancesync.jar. check that you have java installed.");

            let msg = "Copying files from offlineMods and localMods folder to mods folder.";
            println!(" ");
            println!(" ");
            println!("{}", msg.tty_green());
            #[cfg(target_os = "windows")] {
                execute::color("powershell copy-item offlineMods/* mods -ErrorAction Ignore").unwrap();
                execute::color("powershell copy-item localMods/* mods -ErrorAction Ignore").unwrap();
                if server {
                    execute::color("powershell copy-item serverMods/* mods -ErrorAction Ignore").unwrap();
                }
            }
            #[cfg(not(target_os = "windows"))] {
                execute::color("cp -rf offlineMods/* mods").unwrap();
                execute::color("cp -rf localMods/* mods").unwrap();
                if server {
                    execute::color("cp -rf serverMods/* mods").unwrap();
                }
            }
        }

        let msg = "Executing post_exit file if it exists.";
        println!(" ");
        println!(" ");
        println!("{}", msg.tty_green());

        let post_exit_file = "post_exit";

        let files = fs::read_dir(exec_dir.clone()).unwrap()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with(post_exit_file) {
                Some(file_name)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
        if !files.is_empty() {
            match files[0].split('.').last() {
                Some("sh") => execute::default(format!("bash {}", &files[0]).as_str()).unwrap(),
                Some("ps1") => execute::default(format!("powershell {}", &files[0]).as_str()).unwrap(),
                Some(_) => execute::default(&files[0]).unwrap(),
                _ => ()
            };
        }
    }

    
}

#[tokio::main]
async fn main() {
    
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![folder_list, add_folder, get_folder, delete_folder, update_folder, sync_folder])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
