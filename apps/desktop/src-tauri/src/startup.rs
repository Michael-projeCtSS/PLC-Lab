//! Typed launch requests of `PLC Lab.exe`. A launch (a double-click, Explorer's "Open
//! with", a file association or a second instance) may ask to open one project file:
//! `PLC Lab.exe "<file.plclab>"` or `PLC Lab.exe --open-project "<file.plclab>"`. Every
//! other argument is ignored, and nothing from the command line is ever executed or used
//! to build a command: the file is only read and validated by the application use case.
use std::path::{Path, PathBuf};

pub const OPEN_PROJECT: &str = "--open-project";

/// The project file requested by `args` (the program name first): the value of
/// `--open-project <file>`, or else a `.plclab` path given as the first argument (as
/// Explorer passes an opened file). Relative paths are resolved against the launching
/// process's working directory `cwd`.
pub fn open_request(args: &[String], cwd: &Path) -> Option<PathBuf> {
    let value = match args.iter().skip(1).position(|a| a == OPEN_PROJECT) {
        Some(index) => args.get(index + 2)?,
        None => args.get(1).filter(|a| !a.starts_with('-'))?,
    };
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
    fn a_plclab_file_as_first_argument_or_after_the_option_is_accepted() {
        let cwd = Path::new("C:/work");
        assert_eq!(
            open_request(&args(&["app", "--open-project", "D:/p/a.plclab"]), cwd),
            Some(PathBuf::from("D:/p/a.plclab"))
        );
        assert_eq!(
            open_request(&args(&["app", "--open-project", "b.PLCLAB"]), cwd),
            Some(cwd.join("b.PLCLAB"))
        );
        assert_eq!(
            open_request(&args(&["app", "D:/p/My Project.plclab"]), cwd),
            Some(PathBuf::from("D:/p/My Project.plclab"))
        );
        assert_eq!(
            open_request(&args(&["app", "c.plclab"]), cwd),
            Some(cwd.join("c.plclab"))
        );
        for rejected in [
            &["app"][..],
            &["app", "--open-project"],
            &["app", "--open-project", ""],
            &["app", "--open-project", "--other"],
            &["app", "--open-project", "evil.exe"],
            &["app", "--open-project", "notes.txt"],
            &["app", "--open-projectX", "D:/p/a.plclab"],
            &["app", "evil.exe"],
            &["app", "notes.txt"],
            &["app", "-x", "D:/p/a.plclab"],
            &["app", ""],
        ] {
            assert_eq!(open_request(&args(rejected), cwd), None, "{rejected:?}");
        }
    }

    #[test]
    fn the_program_name_is_never_taken_as_the_option() {
        let cwd = Path::new("C:/work");
        assert_eq!(open_request(&args(&["--open-project"]), cwd), None);
        // The first argument is a plain project path here, not the option's value.
        assert_eq!(
            open_request(&args(&["--open-project", "x.plclab"]), cwd),
            Some(cwd.join("x.plclab"))
        );
    }
}
