# Language frontend boundaries

ST is the first frontend workspace crate: a bounded handwritten lexer/parser, dedicated AST and semantic compiler to the existing validated plc-ir. Production depends only on plc-types/plc-ir; runtime/clock are dev-dependencies for actual-source execution proofs. No frontend owns a runtime. LAD/FBD/STL remain reservations. See docs/spec/STRUCTURED_TEXT_ALPHA.md for the exact supported subset.
