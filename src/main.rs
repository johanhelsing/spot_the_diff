use dify::diff::{self};
use glob::glob;
use serde::Serialize;
use std::{collections::HashMap, fs, io::ErrorKind, path::Path};

const BASELINE_FOLDER: &str = "Baseline";
const LATEST_FOLDER: &str = "Latest";
const DIFF_FOLDER: &str = "Diff";

//make enum?
#[derive(Default, Debug, Serialize)]
struct Diff {
    passed: bool,
    failed_image: Option<String>,
    baseline_image: Option<String>,
    diff_image: Option<String>,
}

/// panics if unable to get filename
fn file_name(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .expect("remove prefix")
        .to_str()
        .expect("")
}

fn main() -> Result<(), anyhow::Error> {
    if let Err(e) = fs::remove_dir_all(&DIFF_FOLDER) {
        if e.kind() != ErrorKind::NotFound {
            panic!("failed to remove diff folder {e}");
        }
    }

    let latest_test_folders = glob(&format!("{LATEST_FOLDER}/*"))?;

    // todo: compare and report missing tests

    let mut test_results = HashMap::new();

    for latest_test_folder in latest_test_folders {
        let latest_test_folder = latest_test_folder?;
        let latest_test_folder = latest_test_folder.to_str().unwrap();
        let test_name = file_name(latest_test_folder);

        eprintln!("\n{test_name}:");

        let mut diffs = HashMap::new();

        let baseline_test_folder = format!("{BASELINE_FOLDER}/{test_name}");

        let latest_images = glob(&format!("{latest_test_folder}/*.png"))?;

        for latest_image in latest_images {
            let latest_image = latest_image?;
            let latest_image = latest_image.to_str().unwrap();

            let image_name = file_name(&latest_image);
            let baseline_image = format!("{baseline_test_folder}/{image_name}");
            let image_name_without_ext =
                Path::new(image_name).file_stem().unwrap().to_str().unwrap();
            let diff_test_folder = format!("{DIFF_FOLDER}/{test_name}");
            fs::create_dir_all(&diff_test_folder)?;

            let diff_latest_path =
                format!("{diff_test_folder}/{image_name_without_ext}_latest.png");

            if !Path::new(&baseline_image).exists() {
                eprintln!("        {image_name}: failed, baseline missing");
                diffs.insert(
                    image_name_without_ext.to_string(),
                    Diff {
                        passed: false,
                        failed_image: Some(diff_latest_path.clone()),
                        ..Default::default()
                    },
                );

                fs::copy(&latest_image, &diff_latest_path)?;
                continue;
            }

            let diff_diff_path = format!("{diff_test_folder}/{image_name_without_ext}_diff.png");

            let image_diff = diff::run(&diff::RunParams {
                left: &baseline_image,
                right: &latest_image,
                output: &diff_diff_path,
                threshold: 0.1, // todo: tune
                // output_image_base: Some(OutputImageBase::RightImage),
                output_image_base: None,
                do_not_check_dimensions: false,
                detect_anti_aliased_pixels: false,
                // blend factor appears to be broken...
                // blend_factor_of_unchanged_pixels: Some(0.9),
                blend_factor_of_unchanged_pixels: None,
                block_out_areas: None,
            })
            .expect("dify failed");

            let result = if image_diff.is_some() {
                eprintln!("    FAILED: {image_name}");
                // copy the baseline and latest for convenience
                let diff_baseline_path =
                    format!("{diff_test_folder}/{image_name_without_ext}_baseline.png");
                let diff_latest_path =
                    format!("{diff_test_folder}/{image_name_without_ext}_latest.png");
                fs::copy(latest_image, &diff_latest_path)?;
                fs::copy(baseline_image, &diff_baseline_path)?;

                Diff {
                    passed: false,
                    failed_image: Some(diff_latest_path),
                    baseline_image: Some(diff_baseline_path),
                    diff_image: Some(diff_diff_path),
                }
            } else {
                eprintln!("    ok: {image_name}");
                Diff {
                    passed: true,
                    ..Default::default()
                }
            };

            diffs.insert(image_name_without_ext.to_string(), result);
        }
        test_results.insert(test_name.to_string(), diffs);
    }

    // eprintln!("{test_results:#?}");
    let json = serde_json::to_string_pretty(&test_results)?;
    // println!("{json}");
    fs::write("test_results.json", json)?;

    eprintln!(
        "\n{}/{} passed\n",
        test_results
            .values()
            .map(|t| t.values().filter(|diff| diff.passed).count())
            .sum::<usize>(),
        test_results.values().map(|t| t.len()).sum::<usize>()
    );

    Ok(())
}
