# Language frontend boundaries

ST is the first frontend workspace crate. It contains no parser yet. LAD, FBD, and STL remain directory-level reservations, not dependencies or implementations. All eventual frontends must validate and lower to plc-ir and preserve source mapping. No frontend owns a runtime.
