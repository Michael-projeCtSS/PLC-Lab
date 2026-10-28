//! Typed launch requests. A launch (the root launcher, a file association or a second
//! instance) may ask to open one project file with `--open-project <file.plclab>`. Every
//! other argument is ignored, and nothing from the command line is ever executed or used
//! to build a command: the file is only read and validated by the application use case.
use std::path::{Path, PathBuf};

pub const OPEN_PROJECT: &str = "--open-project";

/// The project file requested by `args` (the program name first), resolved against the
/// launching process's working directory `cwd` when relative.
pub fn open_request(args: &[String], cwd: &Path) -> Option<PathBuf> {
    let position = args.iter().skip(1).position(|a| a == OPEN_PROJECT)? + 1;
    let value = args.get(position + 1)?;
    if value.is_empty() || value.starts_with("--") {
        return None;
    }
    let path = PathBuf::from(value);
    let path = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("plclab"))
        .then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn only_a_plclab_file_after_the_option_is_accepted() {
        let cwd = Path::new("C:/work");
        assert_eq!(
            open_request(&args(&["app", "--open-project", "D:/p/a.plclab"]), cwd),
            Some(PathBuf::from("D:/p/a.plclab"))
        );
        assert_eq!(
            open_request(&args(&["app", "--open-project", "b.PLCLAB"]), cwd),
            Some(cwd.join("b.PLCLAB"))
        );
        for rejected in [
            &["app"][..],
            &["app", "--open-project"],
            &["app", "--open-project", ""],
            &["app", "--open-project", "--other"],
            &["app", "--open-project", "evil.exe"],
            &["app", "--open-project", "notes.txt"],
            &["app", "D:/p/a.plclab"],
            &["app", "--open-projectX", "D:/p/a.plclab"],
        ] {
            assert_eq!(open_request(&args(rejected), cwd), None, "{rejected:?}");
        }
    }

    #[test]
    fn the_program_name_is_never_taken_as_the_option() {
        assert_eq!(
            open_request(&args(&["--open-project", "x.plclab"]), Path::new("C:/work")),
            None
        );
    }
}
