use std::io::BufReader;

use num_traits::FromPrimitive;
use python_marshal::Kind;

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

    for version in common::PYTHON_VERSIONS {
        println!("Testing with Python version: {}", version);
        let pyc_files = common::find_pyc_files(version);

        for pyc_file in pyc_files {
            delete_debug_files();
            println!("Testing pyc file: {:?}", pyc_file);
            let file = std::fs::File::open(&pyc_file).expect("Failed to open pyc file");
            let mut reader = BufReader::new(file);
            
            let code = python_marshal::load_pyc(&mut reader).expect("Failed to read pyc file");
            let original = std::fs::read(&pyc_file).expect("Failed to read pyc file");

            let mut dumped = Vec::new();

            python_marshal::dump_pyc(&mut dumped, code.clone()).expect("Failed to dump pyc file");

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
        }
    }
}
