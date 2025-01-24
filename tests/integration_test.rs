use num_traits::FromPrimitive;
use python_marshal::Kind;

mod common;

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
            println!("Testing pyc file: {:?}", pyc_file);
            let mut file = std::fs::File::open(&pyc_file).expect("Failed to open pyc file");
            let code = python_marshal::load_pyc(&mut file).expect("Failed to read pyc file");
            let original = std::fs::read(&pyc_file).expect("Failed to read pyc file");

            let mut dumped = Vec::new();

            python_marshal::dump_pyc(&mut dumped, code.clone()).expect("Failed to dump pyc file");
            let debug_output = format!("{:#?}", code);
            std::fs::write("debug_output.txt", debug_output).expect("Failed to write debug output to file");

            diff_bytearrays(&original, &dumped)
                .iter()
                .for_each(|(i, a, b)| {
                    println!(
                        "bytearrays differ at index {}: {:?} ({:?}) != {:?} ({:?})",
                        i,
                        a,
                        Kind::from_u8(a & !(Kind::FlagRef as u8)).unwrap_or_else(|| Kind::Unknown),
                        b,
                        Kind::from_u8(b & !(Kind::FlagRef as u8)).unwrap_or_else(|| Kind::Unknown)
                    );

                    let start = if *i >= 10 { *i - 10 } else { 0 };
                    let end = if *i + 10 < original.len() { *i + 10 } else { original.len() - 1 };

                    for j in start..=end {
                        println!(
                            "index {}: original byte {:?} ({:?}), dumped byte {:?} ({:?})",
                            j,
                            original[j],
                            Kind::from_u8(original[j] & !(Kind::FlagRef as u8)).unwrap_or_else(|| Kind::Unknown),
                            dumped[j],
                            Kind::from_u8(dumped[j] & !(Kind::FlagRef as u8)).unwrap_or_else(|| Kind::Unknown)
                        );
                    }

                    assert!(false, "bytearrays differ at index {}", i);
                });

            panic!();
        }
    }
}
