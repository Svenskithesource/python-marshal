use cmd_lib::run_cmd;
use std::fs;
use std::path::{Path, PathBuf};

use python_marshal::magic::PyVersion;

const DATA_PATH: &str = "tests/data";
pub const PYTHON_VERSIONS: &[PyVersion] = &[PyVersion {
    major: 3,
    minor: 10,
    patch: 1,
}];

fn init_repo(version: &PyVersion) {
    // Download the standard library for the specified Python version if it doesn't exist
    if std::fs::metadata(Path::new(DATA_PATH).join(format!("cpython-{}", version))).is_err() {
        let resp = reqwest::blocking::get(format!(
            "https://github.com/python/cpython/archive/refs/tags/v{}.zip",
            version
        ))
        .unwrap();

        let reader = std::io::Cursor::new(resp.bytes().unwrap());
        let mut zip_contents = zip::ZipArchive::new(reader).unwrap();

        for i in 0..zip_contents.len() {
            let mut file = zip_contents.by_index(i).unwrap();
            let outpath = match file.enclosed_name() {
                Some(path) => path,
                None => continue,
            };

            // Only extract files from the `Lib` directory
            if outpath.starts_with(format!("cpython-{}/Lib/", version)) {
                let outpath = Path::new(DATA_PATH).join(outpath);

                if file.is_dir() {
                    fs::create_dir_all(&outpath).unwrap();
                } else {
                    if let Some(parent) = outpath.parent() {
                        if !parent.exists() {
                            fs::create_dir_all(parent).unwrap();
                        }
                    }

                    let mut outfile = fs::File::create(&outpath).unwrap();
                    std::io::copy(&mut file, &mut outfile).unwrap();
                }
            }
        }

        compile_repo(version);
    }
}

fn compile_repo(version: &PyVersion) {
    let lib_dir = Path::new(DATA_PATH).join(format!("cpython-{}/Lib", version));

    let path_str = lib_dir.canonicalize().unwrap();
    let cmd_version = format!("{}.{}", version.major, version.minor);

    let result = run_cmd! {
        py -$cmd_version -m compileall $path_str > nul 2>&1
    };

    if result.is_err() {
        println!(
            "Failed to compile standard library for Python version {}, still continuing",
            cmd_version
        );
    }
}

pub fn setup() {
    for version in PYTHON_VERSIONS {
        println!("Setting up Python version: {}", version);

        std::fs::create_dir_all(DATA_PATH).expect("Failed to create data directory");

        let cmd_version = format!("{}.{}", version.major, version.minor);

        match run_cmd!(
            py -$cmd_version -m __hello__ > nul 2>&1
        ) {
            Ok(_) => {}
            Err(_) => {
                println!("Python version {} is not installed", cmd_version);
                continue;
            }
        }

        init_repo(version); // Initialize the standard library for the specified Python version
    }
}

fn find_pyc_files_in_dir(dir: &Path) -> Vec<PathBuf> {
    let mut pyc_files = Vec::new();

    for entry in fs::read_dir(dir).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();

        if path.is_dir() {
            pyc_files.extend(find_pyc_files_in_dir(&path));
        } else if path.extension().map_or(false, |ext| ext == "pyc") {
            pyc_files.push(path);
        }
    }

    pyc_files
}

/// Recursively find all `.pyc` files in a directory
pub fn find_pyc_files(version: &PyVersion) -> Vec<PathBuf> {
    let mut pyc_files = Vec::new();

    let lib_dir = Path::new(DATA_PATH).join(format!("cpython-{}/Lib", version));
    pyc_files.extend(find_pyc_files_in_dir(&lib_dir));

    pyc_files
}
