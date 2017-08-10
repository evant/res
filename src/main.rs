#[macro_use]
extern crate serde_derive;
extern crate docopt;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::fs::{read_dir, copy, create_dir_all, remove_file};
use std::io::Result;
use std::ascii::AsciiExt;
use std::cmp::max;

use docopt::Docopt;

const VERSION: &'static str = "Version 0.0.1";
const USAGE: &'static str = "
res: android resource management

Usage:
    res ls <type> <source>...
    res cp <type> <source>... <dest>
    res mv <type> <source>... <dest>
    res (-h | --help)
    res --version

Options:
    -h --help   Show this message.
    --version   Show version.
";

#[derive(Debug, Deserialize)]
struct Args {
    cmd_ls: bool,
    cmd_cp: bool,
    cmd_mv: bool,
    arg_type: String,
    arg_source: Vec<String>,
    arg_dest: Option<String>,
    flag_version: bool,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.flag_version {
        println!("{}", VERSION);
        return;
    }

    if args.cmd_ls {
        match ls(&args.arg_type, args.arg_source) {
            Ok(s) => println!("{}", s),
            Err(e) => panic!("{}", e)
        }
        return;
    }

    if args.cmd_cp {
        match cp(&args.arg_type, args.arg_source, args.arg_dest.unwrap()) {
            Ok(_) => {}// success!
            Err(e) => panic!("{}", e)
        }
        return;
    }

    if args.cmd_mv {
        match mv(&args.arg_type, args.arg_source, args.arg_dest.unwrap()) {
            Ok(_) => {}// success!
            Err(e) => panic!("{}", e)
        }
        return;
    }

    println!("{:?}", args);
}

fn ls(res_type: &str, source: Vec<String>) -> Result<String> {
    let files = collect_entries(res_type, source)?;
    let mut longest_name_width = 0;
    let mut used_buckets = HashSet::new();
    let mut buckets_for_name: HashMap<String, Vec<String>> = HashMap::new();

    for (name, _) in &files {
        longest_name_width = max(longest_name_width, name.len());
    }

    for (name, paths) in files {
        let buckets: Vec<String> = paths.iter().map(|path| {
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .and_then(|s| s.rsplit('-').next())
                .unwrap()
                .to_owned()
        }).filter(|s| s != res_type).collect();

        for bucket in buckets.clone() {
            used_buckets.insert(bucket);
        }

        buckets_for_name.insert(name, buckets);
    }

    let sorted_used_buckets: Vec<_> = used_buckets.into_iter().collect();
    let mut sorted_buckets_for_name: Vec<_> = buckets_for_name.into_iter().collect();
    sorted_buckets_for_name.sort_by(|left, right| left.0.cmp(&right.0));

    let mut s = String::new();
    for (name, buckets) in sorted_buckets_for_name {
        s += &format!("{1:0$}", longest_name_width, name);
        for used_bucket in &sorted_used_buckets {
            s += &format!(" {1:0$}", used_bucket.len(), if buckets.contains(&used_bucket) { &used_bucket } else { "" })
        }
        s += "\n";
    }

    Ok(s)
}

fn cp(res_type: &str, source: Vec<String>, dest: String) -> Result<()> {
    cp_or_mv(res_type, source, dest, false)
}

fn mv(res_type: &str, source: Vec<String>, dest: String) -> Result<()> {
    cp_or_mv(res_type, source, dest, true)
}

fn cp_or_mv(res_type: &str, source: Vec<String>, dest: String, delete_source: bool) -> Result<()> {
    let files = collect_entries(res_type, source)?;
    let mut dest_path = Path::new(&dest);
    let file_name;
    if dest_path.is_dir() {
        file_name = None;
    } else {
        file_name = dest_path.file_name();
        dest_path = dest_path.parent().unwrap();
    }

    for (name, paths) in files {
        for path in paths {
            let name = file_name.and_then(|f| f.to_str()).unwrap_or(&name);
            let dest_path = dest_path
                .join(path.parent().and_then(|p| p.file_name()).unwrap())
                .join(&name);

            create_dir_all(&dest_path.parent().unwrap())?;
            copy(&path, &dest_path)?;
            if delete_source {
                remove_file(&path)?;
            }
        }
    }

    Ok(())
}

fn collect_entries(res_type: &str, source: Vec<String>) -> Result<HashMap<String, Vec<PathBuf>>> {
    let mut files = HashMap::new();
    for source in source {
        let mut source_path = Path::new(&source);

        let file_name;
        if source_path.is_dir() {
            file_name = None;
        } else {
            file_name = source_path.file_name();
            source_path = source_path.parent().unwrap();
        }

        let dir = read_dir(source_path)?;
        for entry in dir {
            let path = entry?.path();
            if valid_res_path(&res_type, &path) {
                let dir = read_dir(path)?;
                for entry in dir {
                    let path = entry?.path();
                    if valid_res(&path) {
                        if let Some(file_name) = file_name {
                            if Some(file_name) != path.file_name() {
                                continue;
                            }
                        }
                        if let Some(name) = sanitize_name(&path) {
                            files.entry(name).or_insert_with(|| vec![]).push(path);
                        }
                    }
                }
            }
        }
    }
    return Ok(files);
}

fn valid_res_path<P: AsRef<Path>>(res_type: &str, path: P) -> bool {
    if let Some(name) = path.as_ref().file_name().and_then(|n| n.to_str()) {
        name.starts_with(res_type)
    } else {
        false
    }
}

fn valid_res<P: AsRef<Path>>(path: P) -> bool {
    if let Some(extension) = path.as_ref().extension().and_then(|n| n.to_str()) {
        extension == "png" || extension == "xml"
    } else {
        false
    }
}

fn sanitize_name<P: AsRef<Path>>(path: P) -> Option<String> {
    if let Some(name) = path.as_ref().file_name() {
        Some(name.to_string_lossy().chars()
            .filter_map(|c| {
                if "abcdefghijklmnopqrstuvwxyz1234567890._".contains(c) {
                    Some(c)
                } else if "ABCDEFGHIJKLMNOPQRSTUVWXYZ".contains(c) {
                    Some(c.to_ascii_lowercase())
                } else if " -".contains(c) {
                    Some('_')
                } else {
                    None
                }
            }).collect())
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    extern crate tempdir;

    use super::{ls, cp, mv, valid_res_path, sanitize_name, valid_res};
    use std::fs::{create_dir, File};
    use self::tempdir::TempDir;

    #[test]
    fn valid_res_path_drawable() {
        assert!(valid_res_path("drawable", "drawable"));
    }

    #[test]
    fn valid_res_path_drawable_mdpi() {
        assert!(valid_res_path("drawable", "drawable-mdpi"));
    }

    #[test]
    fn invalid_res_path_layout() {
        assert!(!valid_res_path("drawable", "layout"));
    }

    #[test]
    fn valid_res_path_bad() {
        assert!(!valid_res_path("drawable", "bad"));
    }

    #[test]
    fn valid_res_image_png() {
        assert!(valid_res("image.png"));
    }

    #[test]
    fn valid_res_not_image() {
        assert!(!valid_res("not-image"));
    }

    #[test]
    fn sanitize_name_image() {
        let result = sanitize_name("image.png").unwrap();
        assert_eq!(result, "image.png")
    }

    #[test]
    fn sanitize_name_underscore() {
        let result = sanitize_name("my_image.png").unwrap();
        assert_eq!(result, "my_image.png")
    }

    #[test]
    fn sanitize_name_image_space() {
        let result = sanitize_name("my image.png").unwrap();
        assert_eq!(result, "my_image.png");
    }

    #[test]
    fn ls_single_drawable_png() {
        let tmp = TempDir::new("source").unwrap();
        let source = tmp.path();
        let drawable_mdpi = source.join("drawable-mdpi");
        create_dir(&drawable_mdpi).unwrap();
        File::create(drawable_mdpi.join("image.png")).unwrap();

        let result = ls(
            "drawable",
            vec![source.to_str().unwrap().to_owned()]
        ).unwrap();

        assert_eq!(result, "image.png mdpi\n");
    }

    #[test]
    fn ls_single_drawable_xml() {
        let tmp = TempDir::new("source").unwrap();
        let source = tmp.path();
        let drawable_mdpi = source.join("drawable");
        create_dir(&drawable_mdpi).unwrap();
        File::create(drawable_mdpi.join("image.xml")).unwrap();

        let result = ls(
            "drawable",
            vec![source.to_str().unwrap().to_owned()]
        ).unwrap();

        assert_eq!(result, "image.xml\n");
    }

    #[test]
    fn ls_single_layout_xml() {
        let tmp = TempDir::new("source").unwrap();
        let source = tmp.path();
        let drawable_mdpi = source.join("layout");
        create_dir(&drawable_mdpi).unwrap();
        File::create(drawable_mdpi.join("main.xml")).unwrap();

        let result = ls(
            "layout",
            vec![source.to_str().unwrap().to_owned()]
        ).unwrap();

        assert_eq!(result, "main.xml\n");
    }

    #[test]
    fn ls_single_layout_land_xml() {
        let tmp = TempDir::new("source").unwrap();
        let source = tmp.path();
        let drawable_mdpi = source.join("layout-land");
        create_dir(&drawable_mdpi).unwrap();
        File::create(drawable_mdpi.join("main.xml")).unwrap();

        let result = ls(
            "layout",
            vec![source.to_str().unwrap().to_owned()]
        ).unwrap();

        assert_eq!(result, "main.xml land\n");
    }

    #[test]
    fn cp_single_file_dir() {
        let tmp = TempDir::new("source").unwrap();
        let source = tmp.path();
        let drawable_mdpi = source.join("drawable-mdpi");
        create_dir(&drawable_mdpi).unwrap();
        File::create(drawable_mdpi.join("image.png")).unwrap();
        let tmp = TempDir::new("dest").unwrap();
        let dest = tmp.path();

        cp(
            "drawable",
            vec![source.to_str().unwrap().to_owned()],
            dest.to_str().unwrap().to_owned()
        ).unwrap();

        assert!(dest.join("drawable-mdpi").join("image.png").exists());
    }

    #[test]
    fn cp_single_file() {
        let tmp = TempDir::new("source").unwrap();
        let source = tmp.path();
        let drawable_mdpi = source.join("drawable-mdpi");
        create_dir(&drawable_mdpi).unwrap();
        File::create(drawable_mdpi.join("image1.png")).unwrap();
        File::create(drawable_mdpi.join("image2.png")).unwrap();
        let tmp = TempDir::new("dest").unwrap();
        let dest = tmp.path();

        cp(
            "drawable",
            vec![source.join("image1.png").to_str().unwrap().to_owned()],
            dest.to_str().unwrap().to_owned()
        ).unwrap();

        assert!(source.join("drawable-mdpi").join("image1.png").exists());
        assert!(source.join("drawable-mdpi").join("image2.png").exists());
        assert!(dest.join("drawable-mdpi").join("image1.png").exists());
        assert!(!dest.join("drawable-mdpi").join("image2.png").exists());
    }

    #[test]
    fn cp_single_file_fix_name() {
        let tmp = TempDir::new("source").unwrap();
        let source = tmp.path();
        let drawable_mdpi = source.join("drawable-mdpi");
        create_dir(&drawable_mdpi).unwrap();
        File::create(drawable_mdpi.join("Image1.png")).unwrap();
        let tmp = TempDir::new("dest").unwrap();
        let dest = tmp.path();

        cp(
            "drawable",
            vec![source.join("Image1.png").to_str().unwrap().to_owned()],
            dest.to_str().unwrap().to_owned()
        ).unwrap();

        assert!(source.join("drawable-mdpi").join("Image1.png").exists());
        assert!(dest.join("drawable-mdpi").join("image1.png").exists());
    }

    #[test]
    fn cp_single_file_rename() {
        let tmp = TempDir::new("source").unwrap();
        let source = tmp.path();
        let drawable_mdpi = source.join("drawable-mdpi");
        create_dir(&drawable_mdpi).unwrap();
        File::create(drawable_mdpi.join("image1.png")).unwrap();
        let tmp = TempDir::new("dest").unwrap();
        let dest = tmp.path();

        cp(
            "drawable",
            vec![source.join("image1.png").to_str().unwrap().to_owned()],
            dest.join("image2.png").to_str().unwrap().to_owned()
        ).unwrap();

        assert!(source.join("drawable-mdpi").join("image1.png").exists());
        assert!(dest.join("drawable-mdpi").join("image2.png").exists());
    }

    #[test]
    fn mv_single_file() {
        let tmp = TempDir::new("source").unwrap();
        let source = tmp.path();
        let drawable_mdpi = source.join("drawable-mdpi");
        create_dir(&drawable_mdpi).unwrap();
        File::create(drawable_mdpi.join("image1.png")).unwrap();
        File::create(drawable_mdpi.join("image2.png")).unwrap();
        let tmp = TempDir::new("dest").unwrap();
        let dest = tmp.path();

        mv(
            "drawable",
            vec![source.join("image1.png").to_str().unwrap().to_owned()],
            dest.to_str().unwrap().to_owned()
        ).unwrap();

        assert!(dest.join("drawable-mdpi").join("image1.png").exists());
        assert!(!dest.join("drawable-mdpi").join("image2.png").exists());
        assert!(!source.join("drawable-mdpi").join("image1.png").exists());
        assert!(source.join("drawable-mdpi").join("image2.png").exists());
    }
}
