extern crate winreg;
use keyvalues_parser::{Value, Vdf};
use snailquote::unescape;
use std::fs::{read_dir, read_to_string, remove_dir_all, remove_file};
use std::io::{Error, ErrorKind};
use std::path::Path;
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

fn read_steam_install_path(hive: RegKey) -> String {
    return match hive.get_value("InstallPath") as Result<String, Error> {
        Ok(path) => path,
        Err(_) => panic!("Error: Steam InstallPath not set"),
    };
}

fn vdf_read_libraries<'a>(vdf: &'a Vdf<'a>) -> Option<Vec<String>> {
    let result: Vec<&Value> = match vdf.value.get_obj() {
        Some(x) => x.values().fold(Vec::new(), |mut acc, value| {
            acc.extend(value[0].get_obj().unwrap().get("path").unwrap());
            return acc;
        }),
        None => panic!("at the disco"),
    };
    return Some(
        result
            .into_iter()
            .map::<String, _>(|x: &Value| unescape(&x.to_string()).unwrap())
            .collect::<Vec<String>>(),
    );
}

fn get_workshop_libraries_list(steam_path: String) -> Result<Vec<String>, Error> {
    let libraryfolders_vdf = Path::new(&steam_path)
        .join("steamapps")
        .join("libraryfolders.vdf");
    if !libraryfolders_vdf.exists() {
        return Err(Error::new(
            ErrorKind::NotFound,
            "No library folders present",
        ));
    }
    let mut libraries_list: Vec<String> = Vec::new();
    libraries_list.push(
        Path::new(&steam_path)
            .join("steamapps")
            .join("workshop")
            .into_os_string()
            .into_string()
            .unwrap(),
    );
    let contents = read_to_string(libraryfolders_vdf)?;
    return Vdf::parse(&contents)
        .map_err(|_| Error::new(ErrorKind::Unsupported, "Cannot parse libraries.vdf"))
        .as_ref()
        .map(|vdf| Ok(vdf_read_libraries(vdf).unwrap()))
        .unwrap();
}

fn main() {
    println!("Looking up Steam application");
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let steam_path = match hklm
        .open_subkey("SOFTWARE\\WOW6432Node\\Valve\\Steam")
        .and_then(|hive| Ok(read_steam_install_path(hive)))
        .ok() as Option<String>
    {
        Some(val) => val,
        _ => panic!("Error: Steam not found"),
    };
    println!("Steam is installed @ {}", &steam_path);
    println!("> dropping loginusers.vdf");
    let _ = remove_file(Path::new(&steam_path).join("config").join("loginusers.vdf"));
    println!("> cleaning userdata");
    let _ = read_dir(Path::new(&steam_path).join("userdata")).and_then(|files| {
        files.for_each(|f| {
            let _ = f.and_then(|f| {
                let path = f.path();
                if path.is_dir() {
                    remove_dir_all(path)
                } else {
                    remove_file(path)
                }
            });
        });
        Ok(())
    });
    match get_workshop_libraries_list(steam_path) {
        Ok(libraries) => libraries
            .into_iter()
            .map(|lib| Path::new(&lib).join("steamapps").join("workshop"))
            .filter(move |workshop_path| {
                println!(
                    "> checking workshop directory {}",
                    &(workshop_path.as_path().display())
                );
                return workshop_path.exists();
            })
            .for_each(move |workshop_path| {
                if let Ok(dir) = read_dir(&workshop_path) {
                    for entry in dir {
                        if let Ok(entry) = entry {
                            let path = entry.path();
                            let exception_message = "Failed to remove ".to_owned()
                                + &(path.as_path().display().to_string().to_owned());
                            println!(
                                "--> removing {}",
                                &(path.as_path().display().to_string().to_owned())
                            );
                            if path.is_dir() {
                                remove_dir_all(path).expect(&exception_message);
                            } else {
                                remove_file(path).expect(&exception_message);
                            }
                        }
                    }
                }
            }),
        Err(e) => panic!("{}", e.to_string()),
    }
}
