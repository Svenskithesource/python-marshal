use common::DATA_PATH;
use rayon::prelude::*;
use std::{
    io::BufReader,
    path::{Path, PathBuf},
};

use num_traits::FromPrimitive;
use python_marshal::{
    dump_bytes, magic::PyVersion, minimize_references, optimize_references,
    resolver::resolve_all_refs, Kind, PycFile,
};

mod common;

fn delete_debug_files() {
    let _ = std::fs::remove_file("debug_output.txt");
    let _ = std::fs::remove_file("write_log.txt");
    let _ = std::fs::remove_file("read_log.txt");
}

fn diff_bytearrays(a: &[u8], b: &[u8]) -> Vec<(usize, u8, u8)> {
    let mut diff = Vec::new();
    for (i, (&byte_a, &byte_b)) in a.iter().zip(b.iter()).enumerate() {
        if byte_a != byte_b {
            diff.push((i, byte_a, byte_b));
        }
    }
    diff
}

#[test]
fn test_recompile_standard_lib() {
    common::setup();
    env_logger::init();

    common::PYTHON_VERSIONS.par_iter().for_each(|version| {
        println!("Testing with Python version: {}", version);
        let pyc_files = common::find_pyc_files(version);

        pyc_files.par_iter().for_each(|pyc_file| {
            delete_debug_files();
            println!("Testing pyc file: {:?}", pyc_file);
            let file = std::fs::File::open(&pyc_file).expect("Failed to open pyc file");
            let mut reader = BufReader::new(file);

            let code = python_marshal::load_pyc(&mut reader).expect("Failed to read pyc file");
            let original = std::fs::read(&pyc_file).expect("Failed to read pyc file");

            let (temp_obj, temp_refs) = optimize_references(&code.object, &code.references);

            dump_bytes(temp_obj, Some(temp_refs), code.python_version, 4)
                .expect("Failed to dump bytes");

            let (temp_obj, temp_refs) = resolve_all_refs(&code.object, &code.references);

            assert_eq!(temp_refs.len(), 0);

            dump_bytes(temp_obj, Some(temp_refs), code.python_version, 4)
                .expect("Failed to dump bytes");

            assert_eq!(code, code);

            let dumped = python_marshal::dump_pyc(code.clone()).expect("Failed to dump pyc file");

            if original != dumped {
                let debug_output = format!("{:#?}", code);
                std::fs::write("debug_output.txt", debug_output)
                    .expect("Failed to write debug output to file");

                const CONTEXT_SIZE: usize = 50;

                diff_bytearrays(&original, &dumped)
                    .iter()
                    .for_each(|(i, a, b)| {
                        println!(
                            "bytearrays differ at index {}: {:?} ({:?}) != {:?} ({:?})",
                            i,
                            a,
                            Kind::from_u8(a & !(Kind::FlagRef as u8))
                                .unwrap_or_else(|| Kind::Unknown),
                            b,
                            Kind::from_u8(b & !(Kind::FlagRef as u8))
                                .unwrap_or_else(|| Kind::Unknown)
                        );

                        let start = if *i >= CONTEXT_SIZE {
                            *i - CONTEXT_SIZE
                        } else {
                            0
                        };
                        let end = if *i + CONTEXT_SIZE < original.len() {
                            *i + CONTEXT_SIZE
                        } else {
                            original.len() - 1
                        };

                        for j in start..=end {
                            println!(
                                "index {}: original byte {:?} ({:?}), dumped byte {:?} ({:?})",
                                j,
                                original[j],
                                Kind::from_u8(original[j] & !(Kind::FlagRef as u8))
                                    .unwrap_or_else(|| Kind::Unknown),
                                dumped[j],
                                Kind::from_u8(dumped[j] & !(Kind::FlagRef as u8))
                                    .unwrap_or_else(|| Kind::Unknown)
                            );
                        }

                        assert!(false, "bytearrays differ at index {}", i);
                    });
            }
        });
    });
}

fn get_custom_path(original_path: &Path, version: &PyVersion, prefix: &'static str) -> PathBuf {
    let relative_path = original_path
        .strip_prefix(Path::new(DATA_PATH).join(format!("cpython-{}/Lib", version)))
        .unwrap();
    Path::new(DATA_PATH)
        .join(format!("{prefix}-{version}/Lib"))
        .join(relative_path)
}

#[test]
#[ignore = "This test will write the resolved files to disk so we can run the Python tests on them. That way we're sure the resolved files are correct."]
fn test_write_resolved_standard_lib() {
    common::setup();
    env_logger::init();

    common::PYTHON_VERSIONS.par_iter().for_each(|version| {
        println!("Testing with Python version: {}", version);
        let pyc_files = common::find_pyc_files(version);

        pyc_files.par_iter().for_each(|pyc_file| {
            delete_debug_files();
            println!("Testing pyc file: {:?}", pyc_file);
            let file = std::fs::File::open(&pyc_file).expect("Failed to open pyc file");
            let mut reader = BufReader::new(file);

            let code: PycFile =
                python_marshal::load_pyc(&mut reader).expect("Failed to read pyc file");

            let (temp_obj, temp_refs) = resolve_all_refs(&code.object, &code.references);

            assert_eq!(temp_refs.len(), 0);

            let dumped_pyc = PycFile {
                python_version: code.python_version,
                hash: code.hash,
                timestamp: code.timestamp,
                object: temp_obj,
                references: temp_refs,
            };

            let output_dir = get_custom_path(&pyc_file.parent().unwrap(), version, "resolved")
                .parent()
                .unwrap()
                .to_path_buf();

            std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");

            let output_path = Path::new(&output_dir).join(pyc_file.file_name().unwrap());

            let mut output_file =
                std::fs::File::create(&output_path).expect("Failed to create output file");

            std::io::copy(
                &mut output_file,
                &mut python_marshal::dump_pyc(dumped_pyc).expect("Failed to dump pyc file"),
            )
            .expect("Failed to write to the file");
        });
    });
}

#[test]
#[ignore = "This test will write the optimized files to disk so we can run the Python tests on them. That way we're sure the optimized files are correct."]
fn test_write_optimized_standard_lib() {
    common::setup();
    env_logger::init();

    common::PYTHON_VERSIONS.par_iter().for_each(|version| {
        println!("Testing with Python version: {}", version);
        let pyc_files = common::find_pyc_files(version);

        pyc_files.par_iter().for_each(|pyc_file| {
            delete_debug_files();
            println!("Testing pyc file: {:?}", pyc_file);
            let file = std::fs::File::open(&pyc_file).expect("Failed to open pyc file");
            let mut reader = BufReader::new(file);

            let code: PycFile =
                python_marshal::load_pyc(&mut reader).expect("Failed to read pyc file");

            let (temp_obj, temp_refs) = resolve_all_refs(&code.object, &code.references);

            assert_eq!(temp_refs.len(), 0);

            let (temp_obj, temp_refs) = minimize_references(&temp_obj, temp_refs);

            let dumped_pyc = PycFile {
                python_version: code.python_version,
                hash: code.hash,
                timestamp: code.timestamp,
                object: temp_obj,
                references: temp_refs,
            };

            let output_dir = get_custom_path(&pyc_file.parent().unwrap(), version, "optimized")
                .parent()
                .unwrap()
                .to_path_buf();

            std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");

            let output_path = Path::new(&output_dir).join(pyc_file.file_name().unwrap());

            let mut output_file =
                std::fs::File::create(&output_path).expect("Failed to create output file");

            std::io::copy(
                &mut output_file,
                &mut python_marshal::dump_pyc(dumped_pyc).expect("Failed to dump pyc file"),
            )
            .expect("Failed to write to the file");
        });
    });
}
