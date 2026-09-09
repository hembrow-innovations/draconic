//! Path and filesystem host API registry rows (H03, H04).

use super::{HostApiEntry, HostAvailability};

pub(super) const ENTRIES: &[HostApiEntry] = &[
    HostApiEntry {
        name: "pathJoin",
        availability: HostAvailability::BOTH,
        note: "H03.01 path join",
    },
    HostApiEntry {
        name: "pathNormalize",
        availability: HostAvailability::BOTH,
        note: "H03.01 path normalize",
    },
    HostApiEntry {
        name: "pathDirname",
        availability: HostAvailability::BOTH,
        note: "H03.02 path dirname",
    },
    HostApiEntry {
        name: "pathBasename",
        availability: HostAvailability::BOTH,
        note: "H03.02 path basename",
    },
    HostApiEntry {
        name: "pathExtname",
        availability: HostAvailability::BOTH,
        note: "H03.02 path extname",
    },
    HostApiEntry {
        name: "pathIsAbsolute",
        availability: HostAvailability::BOTH,
        note: "H03.02 path isAbsolute",
    },
    HostApiEntry {
        name: "pathResolve",
        availability: HostAvailability::BOTH,
        note: "H03.03 path resolve (cwd-relative)",
    },
    HostApiEntry {
        name: "readFileText",
        availability: HostAvailability::BOTH,
        note: "H04.01 file read text",
    },
    HostApiEntry {
        name: "readFileBytes",
        availability: HostAvailability::BOTH,
        note: "H04.01 file read bytes",
    },
    HostApiEntry {
        name: "writeFileText",
        availability: HostAvailability::BOTH,
        note: "H04.02 file write text",
    },
    HostApiEntry {
        name: "writeFileBytes",
        availability: HostAvailability::BOTH,
        note: "H04.02 file write bytes",
    },
    HostApiEntry {
        name: "appendFileText",
        availability: HostAvailability::BOTH,
        note: "H04.02 file append text",
    },
    HostApiEntry {
        name: "appendFileBytes",
        availability: HostAvailability::BOTH,
        note: "H04.02 file append bytes",
    },
    HostApiEntry {
        name: "exists",
        availability: HostAvailability::BOTH,
        note: "H04.03 path exists",
    },
    HostApiEntry {
        name: "stat",
        availability: HostAvailability::BOTH,
        note: "H04.03 path stat",
    },
    HostApiEntry {
        name: "mkdir",
        availability: HostAvailability::BOTH,
        note: "H04.04 mkdir",
    },
    HostApiEntry {
        name: "mkdirAll",
        availability: HostAvailability::BOTH,
        note: "H04.04 mkdir recursive",
    },
    HostApiEntry {
        name: "readdir",
        availability: HostAvailability::BOTH,
        note: "H04.04 readdir",
    },
    HostApiEntry {
        name: "rmdir",
        availability: HostAvailability::BOTH,
        note: "H04.04 rmdir",
    },
    HostApiEntry {
        name: "removeFile",
        availability: HostAvailability::BOTH,
        note: "H04.04 remove file",
    },
    HostApiEntry {
        name: "renameFile",
        availability: HostAvailability::BOTH,
        note: "H04.05 rename file",
    },
    HostApiEntry {
        name: "copyFile",
        availability: HostAvailability::BOTH,
        note: "H04.05 copy file",
    },
    HostApiEntry {
        name: "openFile",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H04.06 open file handle",
    },
    HostApiEntry {
        name: "fileRead",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H04.06 file handle read",
    },
    HostApiEntry {
        name: "fileWrite",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H04.06 file handle write",
    },
    HostApiEntry {
        name: "fileSeek",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H04.06 file handle seek",
    },
    HostApiEntry {
        name: "closeFile",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H04.06 close file handle",
    },
];

#[cfg(test)]
mod tests {
    use super::super::{is_available, lookup, unsupported_diagnostic, CompileTarget};
    use draconic_diagnostics::Span;

    #[test]
    fn registry_lists_path_join_normalize_both() {
        for name in ["pathJoin", "pathNormalize"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_path_dirname_basename_extname_is_absolute_both() {
        for name in [
            "pathDirname",
            "pathBasename",
            "pathExtname",
            "pathIsAbsolute",
        ] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_path_resolve_both() {
        let name = "pathResolve";
        let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
        assert!(entry.availability.js, "{name}");
        assert!(entry.availability.native, "{name}");
        assert!(is_available(name, CompileTarget::Js), "{name}");
        assert!(is_available(name, CompileTarget::Native), "{name}");
        assert!(
            unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
            "{name}"
        );
    }

    #[test]
    fn registry_lists_read_file_both() {
        for name in ["readFileText", "readFileBytes"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_write_file_both() {
        for name in [
            "writeFileText",
            "writeFileBytes",
            "appendFileText",
            "appendFileBytes",
        ] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_exists_stat_both() {
        for name in ["exists", "stat"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_dir_ops_both() {
        for name in ["mkdir", "mkdirAll", "readdir", "rmdir", "removeFile"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_rename_copy_both() {
        for name in ["renameFile", "copyFile"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_open_handle_native_only() {
        for name in ["openFile", "fileRead", "fileWrite", "fileSeek", "closeFile"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(!entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(!is_available(name, CompileTarget::Js), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_some(),
                "{name}"
            );
        }
    }
}
